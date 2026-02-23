use super::error::AppError;
use super::model::{
    AuditEventRecord, ComplianceAttestationRecord, ComplianceRequestRecord, NormalizedSubject,
    ProviderResponseReference, SubjectType,
};
use super::schema::{
    ComplianceDecision, IntakeComplianceRequest, IntakeComplianceResponse, SubjectInput,
};
use crate::app::AppState;
use crate::infra::mongo::{
    ATTESTATIONS_COLLECTION, AUDIT_EVENTS_COLLECTION, PROVIDER_REFS_COLLECTION, REQUESTS_COLLECTION,
};
use crate::infra::redis::{IDEMPOTENCY_PREFIX, JOB_STATUS_PREFIX, SCREEN_CACHE_PREFIX};
use crate::service::attestation_hash_service::{build_attestation_id, compute_attestation_hash};
use crate::service::confidential_http_service::{FxQuote, fetch_fx_quote};
use crate::service::encryption_service::encrypt_for_storage;
use crate::service::policy_eval_service::{evaluate_intake_policy, load_policy_snapshot};
use crate::service::sanctions_service::{
    ScreeningHit, ScreeningResult, load_sanctions_entries, screen_subjects,
};
use crate::service::signature_service::verify_internal_signature;
use mongodb::Collection;
use mongodb::bson::doc;
use redis::AsyncCommands;
use redis::Script;
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn intake_and_normalize(
    state: &AppState,
    req: IntakeComplianceRequest,
) -> Result<IntakeComplianceResponse, AppError> {
    validate_request(state, &req)?;
    maybe_verify_signature(state, &req)?;

    let request_hash = compute_request_hash(&req)?;
    let normalized_subjects = normalize_subjects(&req.subjects)?;
    let policy_snapshot = load_policy_snapshot(&state.config.policy_snapshot_path)
        .map_err(|e| AppError::bad_request("POLICY_SNAPSHOT_ERROR", e))?;
    if policy_snapshot.active.version != state.config.policy_version {
        return Err(AppError::bad_request(
            "POLICY_VERSION_NOT_APPROVED",
            format!(
                "policy version mismatch: configured={} active={}",
                state.config.policy_version, policy_snapshot.active.version
            ),
        ));
    }

    if let Some(infra) = &state.infra {
        if let Some(existing) =
            check_idempotent_existing(infra, &req.request_id, &request_hash).await?
        {
            write_audit_event(
                infra,
                &req.request_id,
                Some(existing.attestation_id.clone()),
                "IDEMPOTENT_REPLAY",
                "COMPLETED",
                None,
            )
            .await?;
            return Ok(existing);
        }
        reserve_replay_keys(
            infra,
            &req.nonce,
            &request_hash,
            state.config.replay_ttl_seconds,
        )
        .await?;
    }

    let sanctions_entries = load_sanctions_entries(&state.config.sanctions_data_path)
        .map_err(|e| AppError::bad_request("SANCTIONS_DATA_ERROR", e))?;
    let screening = if let Some(infra) = &state.infra {
        get_or_compute_screening(
            infra,
            &normalized_subjects,
            &sanctions_entries,
            &policy_snapshot.active.hash,
        )
        .await?
    } else {
        screen_subjects(&normalized_subjects, &sanctions_entries)
    };

    let (decision, risk_score) = evaluate_intake_policy(&screening, &policy_snapshot.thresholds);
    let fx_quote = if state.config.fx_lookup_enabled {
        fetch_fx_quote(
            &state.config.frankfurter_base_url,
            &state.config.fx_base_currency,
            &state.config.fx_quote_currency,
        )
        .await
        .ok()
    } else {
        None
    };
    let issued_at = req.timestamp;
    let expires_at = req
        .timestamp
        .checked_add(state.config.attestation_ttl_seconds)
        .ok_or_else(|| {
            AppError::bad_request("ATTESTATION_TIME_ERROR", "invalid expires_at value")
        })?;
    let attestation_hash = compute_attestation_hash(
        &req.workflow_run_id,
        &req.request_id,
        &policy_snapshot.active.version,
        &policy_snapshot.active.hash,
        decision,
        risk_score,
        issued_at,
        expires_at,
        screening.hits.len(),
        &normalized_subjects,
        &screening.match_digest,
    );
    let attestation_id = build_attestation_id(&attestation_hash);
    let response = IntakeComplianceResponse {
        attestation_id: attestation_id.clone(),
        workflow_run_id: req.workflow_run_id.clone(),
        request_id: req.request_id.clone(),
        accepted: true,
        normalized_subject_count: normalized_subjects.len(),
        normalized_subjects: normalized_subjects.clone(),
        policy_version: policy_snapshot.active.version.clone(),
        policy_hash: policy_snapshot.active.hash.clone(),
        decision,
        risk_score,
        sanctions_hit_count: screening.hits.len(),
        attestation_hash: attestation_hash.clone(),
        issued_at,
        expires_at,
        error_code: None,
        reason: "accepted".to_string(),
        fx_quote,
    };

    if let Some(infra) = &state.infra {
        persist_records(
            state,
            infra,
            &response,
            req.timestamp,
            &req.nonce,
            &request_hash,
            &screening,
            response.fx_quote.as_ref(),
        )
        .await?;
        finalize_job_keys(
            infra,
            &req.request_id,
            &attestation_id,
            state.config.replay_ttl_seconds,
        )
        .await?;
        write_audit_event(
            infra,
            &req.request_id,
            Some(attestation_id),
            "ATTESTATION_FINALIZED",
            "COMPLETED",
            Some(format!(
                "decision={} risk_score={} hits={}",
                response.decision.as_str(),
                response.risk_score,
                response.sanctions_hit_count
            )),
        )
        .await?;
    }

    Ok(response)
}

pub async fn get_attestation_by_id(
    state: &AppState,
    attestation_id: &str,
) -> Result<IntakeComplianceResponse, AppError> {
    let Some(infra) = &state.infra else {
        return Err(AppError::bad_request(
            "PERSISTENCE_DISABLED",
            "persistence is not configured",
        ));
    };

    let attestations: Collection<ComplianceAttestationRecord> =
        infra.mongo_db.collection(ATTESTATIONS_COLLECTION);
    let found = attestations
        .find_one(doc! { "attestation_id": attestation_id })
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("mongo find failed: {e}")))?;
    let Some(record) = found else {
        return Err(AppError::not_found(
            "ATTESTATION_NOT_FOUND",
            "attestation not found",
        ));
    };

    Ok(IntakeComplianceResponse {
        attestation_id: record.attestation_id,
        workflow_run_id: record.workflow_run_id,
        request_id: record.request_id,
        accepted: true,
        normalized_subject_count: record.normalized_subjects.len(),
        normalized_subjects: record.normalized_subjects,
        policy_version: record.policy_version,
        policy_hash: record.policy_hash,
        decision: decision_from_str(&record.decision),
        risk_score: record.risk_score,
        sanctions_hit_count: record.sanctions_hit_count,
        attestation_hash: record.attestation_hash,
        issued_at: record.issued_at,
        expires_at: record.expires_at,
        error_code: None,
        reason: "accepted".to_string(),
        fx_quote: record.fx_quote,
    })
}

fn validate_request(state: &AppState, req: &IntakeComplianceRequest) -> Result<(), AppError> {
    if state.config.attestation_ttl_seconds <= 0 {
        return Err(AppError::bad_request(
            "INVALID_ATTESTATION_TTL",
            "attestation ttl must be positive",
        ));
    }
    if state.config.replay_ttl_seconds <= 0 {
        return Err(AppError::bad_request(
            "INVALID_REPLAY_TTL",
            "replay ttl must be positive",
        ));
    }
    if req.workflow_run_id.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_WORKFLOW_RUN_ID",
            "workflow_run_id is required",
        ));
    }
    if req.request_id.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_REQUEST_ID",
            "request_id is required",
        ));
    }
    if req.nonce.trim().is_empty() {
        return Err(AppError::bad_request("INVALID_NONCE", "nonce is required"));
    }
    if req.subjects.is_empty() {
        return Err(AppError::bad_request(
            "INVALID_SUBJECTS",
            "at least one subject is required",
        ));
    }
    if req.timestamp <= 0 {
        return Err(AppError::bad_request(
            "INVALID_TIMESTAMP",
            "timestamp must be a positive unix epoch",
        ));
    }

    let now = now_unix()?;
    if req.timestamp < now - state.config.intake_max_age_seconds {
        return Err(AppError::bad_request(
            "REQUEST_EXPIRED",
            "request timestamp is older than allowed window",
        ));
    }
    if req.timestamp > now + state.config.intake_max_future_skew_seconds {
        return Err(AppError::bad_request(
            "TIMESTAMP_IN_FUTURE",
            "request timestamp exceeds future skew allowance",
        ));
    }
    Ok(())
}

fn maybe_verify_signature(state: &AppState, req: &IntakeComplianceRequest) -> Result<(), AppError> {
    if !state.config.require_internal_signature {
        return Ok(());
    }
    let signing_secret = state
        .config
        .internal_signing_secret
        .as_deref()
        .ok_or_else(|| {
            AppError::internal(
                "SIGNING_CONFIG_MISSING",
                "INTERNAL_SIGNING_SECRET is required when signature auth is enabled",
            )
        })?;
    let signature = req.internal_signature.as_deref().ok_or_else(|| {
        AppError::unauthorized("MISSING_SIGNATURE", "internal_signature is required")
    })?;
    let signing_payload = canonical_signing_payload(req)?;
    verify_internal_signature(&signing_payload, signature, signing_secret)
        .map_err(|e| AppError::unauthorized("BAD_SIGNATURE", e))
}

fn canonical_signing_payload(req: &IntakeComplianceRequest) -> Result<String, AppError> {
    #[derive(Serialize)]
    struct Payload<'a> {
        workflow_run_id: &'a str,
        request_id: &'a str,
        nonce: &'a str,
        timestamp: i64,
        subjects: &'a [SubjectInput],
    }
    serde_json::to_string(&Payload {
        workflow_run_id: &req.workflow_run_id,
        request_id: &req.request_id,
        nonce: &req.nonce,
        timestamp: req.timestamp,
        subjects: &req.subjects,
    })
    .map_err(|e| AppError::internal("SERIALIZATION_ERROR", format!("sign payload failed: {e}")))
}

fn compute_request_hash(req: &IntakeComplianceRequest) -> Result<String, AppError> {
    let payload = canonical_signing_payload(req)?;
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn now_unix() -> Result<i64, AppError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal("CLOCK_ERROR", format!("clock error: {e}")))?;
    Ok(elapsed.as_secs() as i64)
}

fn normalize_subjects(subjects: &[SubjectInput]) -> Result<Vec<NormalizedSubject>, AppError> {
    let mut normalized = Vec::with_capacity(subjects.len());
    for subject in subjects {
        let has_counterparty = subject.counterparty.is_some();
        let has_entity = subject.entity.is_some();
        if has_counterparty == has_entity {
            return Err(AppError::bad_request(
                "INVALID_SUBJECT_SHAPE",
                "each subject must provide exactly one of counterparty or entity",
            ));
        }

        if let Some(c) = &subject.counterparty {
            if c.counterparty_id.trim().is_empty() {
                return Err(AppError::bad_request(
                    "INVALID_COUNTERPARTY",
                    "counterparty_id is required",
                ));
            }
            normalized.push(NormalizedSubject {
                subject_id: c.counterparty_id.clone(),
                subject_type: SubjectType::Counterparty,
                jurisdiction: c.country.clone(),
                address: c.wallet_address.clone(),
                legal_name: None,
            });
            continue;
        }

        if let Some(e) = &subject.entity {
            if e.entity_id.trim().is_empty() {
                return Err(AppError::bad_request(
                    "INVALID_ENTITY",
                    "entity_id is required",
                ));
            }
            normalized.push(NormalizedSubject {
                subject_id: e.entity_id.clone(),
                subject_type: SubjectType::Entity,
                jurisdiction: e.registration_country.clone(),
                address: None,
                legal_name: e.legal_name.clone(),
            });
        }
    }
    Ok(normalized)
}

async fn check_idempotent_existing(
    infra: &crate::infra::InfraClients,
    request_id: &str,
    request_hash: &str,
) -> Result<Option<IntakeComplianceResponse>, AppError> {
    let requests: Collection<ComplianceRequestRecord> =
        infra.mongo_db.collection(REQUESTS_COLLECTION);
    let existing = requests
        .find_one(doc! { "request_id": request_id })
        .await
        .map_err(|e| {
            AppError::internal(
                "PERSISTENCE_ERROR",
                format!("mongo find request failed: {e}"),
            )
        })?;
    let Some(existing) = existing else {
        return Ok(None);
    };
    if existing.request_hash != request_hash {
        return Err(AppError::bad_request(
            "IDEMPOTENCY_CONFLICT",
            "same request_id used with different payload",
        ));
    }

    let attestations: Collection<ComplianceAttestationRecord> =
        infra.mongo_db.collection(ATTESTATIONS_COLLECTION);
    let att = attestations
        .find_one(doc! { "request_id": request_id })
        .await
        .map_err(|e| {
            AppError::internal(
                "PERSISTENCE_ERROR",
                format!("mongo find attestation failed: {e}"),
            )
        })?;
    let Some(att) = att else {
        return Err(AppError::internal(
            "IDEMPOTENCY_STATE_ERROR",
            "request exists but attestation record missing",
        ));
    };

    Ok(Some(IntakeComplianceResponse {
        attestation_id: att.attestation_id,
        workflow_run_id: att.workflow_run_id,
        request_id: att.request_id,
        accepted: true,
        normalized_subject_count: att.normalized_subjects.len(),
        normalized_subjects: att.normalized_subjects,
        policy_version: att.policy_version,
        policy_hash: att.policy_hash,
        decision: decision_from_str(&att.decision),
        risk_score: att.risk_score,
        sanctions_hit_count: att.sanctions_hit_count,
        attestation_hash: att.attestation_hash,
        issued_at: att.issued_at,
        expires_at: att.expires_at,
        error_code: None,
        reason: "accepted".to_string(),
        fx_quote: att.fx_quote,
    }))
}

async fn reserve_replay_keys(
    infra: &crate::infra::InfraClients,
    nonce: &str,
    request_hash: &str,
    ttl_seconds: i64,
) -> Result<(), AppError> {
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connect failed: {e}")))?;

    let nonce_key = format!("replay:nonce:{nonce}");
    let hash_key = format!("replay:reqhash:{request_hash}");
    let script = Script::new(
        r#"
        local nonceKey = KEYS[1]
        local hashKey = KEYS[2]
        local ttl = tonumber(ARGV[1])
        if redis.call('EXISTS', nonceKey) == 1 then return 1 end
        if redis.call('EXISTS', hashKey) == 1 then return 2 end
        redis.call('SET', nonceKey, '1', 'EX', ttl, 'NX')
        redis.call('SET', hashKey, '1', 'EX', ttl, 'NX')
        return 0
    "#,
    );

    let status: i32 = script
        .key(&nonce_key)
        .key(&hash_key)
        .arg(ttl_seconds)
        .invoke_async(&mut conn)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("replay script failed: {e}")))?;

    match status {
        0 => Ok(()),
        1 => Err(AppError::bad_request("REPLAY_NONCE", "nonce already used")),
        2 => Err(AppError::bad_request(
            "REPLAY_REQUEST_HASH",
            "request hash already used",
        )),
        _ => Err(AppError::internal(
            "REDIS_ERROR",
            "unexpected replay script status",
        )),
    }
}

async fn get_or_compute_screening(
    infra: &crate::infra::InfraClients,
    subjects: &[NormalizedSubject],
    sanctions_entries: &[crate::service::sanctions_service::SanctionsEntry],
    policy_hash: &str,
) -> Result<ScreeningResult, AppError> {
    let cache_key = build_screen_cache_key(subjects, policy_hash);
    let redis_key = format!("{SCREEN_CACHE_PREFIX}{cache_key}");
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connect failed: {e}")))?;

    let cached: Option<String> = conn
        .get(&redis_key)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis get cache failed: {e}")))?;
    if let Some(raw) = cached {
        let parsed = serde_json::from_str::<CachedScreening>(&raw).map_err(|e| {
            AppError::internal("REDIS_ERROR", format!("cached screening parse failed: {e}"))
        })?;
        return Ok(ScreeningResult {
            hits: parsed.hits,
            match_digest: parsed.match_digest,
        });
    }

    let screening = screen_subjects(subjects, sanctions_entries);
    let payload = CachedScreening {
        hits: screening.hits.clone(),
        match_digest: screening.match_digest.clone(),
    };
    let raw = serde_json::to_string(&payload)
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("cache serialize failed: {e}")))?;
    let _: () = conn
        .set_ex(redis_key, raw, 300)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis set cache failed: {e}")))?;
    Ok(screening)
}

fn build_screen_cache_key(subjects: &[NormalizedSubject], policy_hash: &str) -> String {
    let mut stable = subjects
        .iter()
        .map(|s| {
            format!(
                "{}:{:?}:{:?}:{:?}:{:?}",
                s.subject_id, s.subject_type, s.jurisdiction, s.address, s.legal_name
            )
        })
        .collect::<Vec<_>>();
    stable.sort();
    let mut hasher = Sha256::new();
    for line in stable {
        hasher.update(line.as_bytes());
        hasher.update(b"|");
    }
    hasher.update(policy_hash.as_bytes());
    hex::encode(hasher.finalize())
}

async fn persist_records(
    state: &AppState,
    infra: &crate::infra::InfraClients,
    response: &IntakeComplianceResponse,
    request_timestamp: i64,
    nonce: &str,
    request_hash: &str,
    screening: &ScreeningResult,
    fx_quote: Option<&FxQuote>,
) -> Result<(), AppError> {
    let requests: Collection<ComplianceRequestRecord> =
        infra.mongo_db.collection(REQUESTS_COLLECTION);
    let provider_refs: Collection<ProviderResponseReference> =
        infra.mongo_db.collection(PROVIDER_REFS_COLLECTION);
    let attestations: Collection<ComplianceAttestationRecord> =
        infra.mongo_db.collection(ATTESTATIONS_COLLECTION);

    let request_doc = ComplianceRequestRecord {
        request_id: response.request_id.clone(),
        nonce: nonce.to_string(),
        request_hash: request_hash.to_string(),
        workflow_run_id: response.workflow_run_id.clone(),
        received_at: now_unix()?,
        request_timestamp,
        policy_version: response.policy_version.clone(),
        policy_hash: response.policy_hash.clone(),
    };

    let encryption_key = state.config.encryption_key_hex.as_deref().ok_or_else(|| {
        AppError::internal(
            "ENCRYPTION_CONFIG_MISSING",
            "ENCRYPTION_KEY_HEX required for provider payload persistence",
        )
    })?;
    let raw_provider_payload = serde_json::to_string(&serde_json::json!({
        "sanctions_hits": &screening.hits,
        "fx_quote": fx_quote,
    }))
    .map_err(|e| {
        AppError::internal(
            "SERIALIZATION_ERROR",
            format!("provider payload encode failed: {e}"),
        )
    })?;
    let encrypted_ref = encrypt_for_storage(&raw_provider_payload, encryption_key)
        .map_err(|e| AppError::internal("ENCRYPTION_ERROR", e))?;
    let provider_ref = ProviderResponseReference {
        request_id: response.request_id.clone(),
        provider_ref_id: format!("prov_{}", &response.attestation_id),
        source: "local_sanctions_dataset+frankfurter".to_string(),
        redacted_payload_ref: encrypted_ref,
        created_at: now_unix()?,
    };

    let att_doc = ComplianceAttestationRecord {
        attestation_id: response.attestation_id.clone(),
        request_id: response.request_id.clone(),
        workflow_run_id: response.workflow_run_id.clone(),
        policy_version: response.policy_version.clone(),
        policy_hash: response.policy_hash.clone(),
        decision: response.decision.as_str().to_string(),
        risk_score: response.risk_score,
        attestation_hash: response.attestation_hash.clone(),
        issued_at: response.issued_at,
        expires_at: response.expires_at,
        sanctions_hit_count: response.sanctions_hit_count,
        normalized_subjects: response.normalized_subjects.clone(),
        fx_quote: response.fx_quote.clone(),
    };

    requests.insert_one(request_doc).await.map_err(|e| {
        AppError::internal("PERSISTENCE_ERROR", format!("insert request failed: {e}"))
    })?;
    provider_refs.insert_one(provider_ref).await.map_err(|e| {
        AppError::internal(
            "PERSISTENCE_ERROR",
            format!("insert provider ref failed: {e}"),
        )
    })?;
    attestations.insert_one(att_doc).await.map_err(|e| {
        AppError::internal(
            "PERSISTENCE_ERROR",
            format!("insert attestation failed: {e}"),
        )
    })?;

    write_audit_event(
        infra,
        &response.request_id,
        Some(response.attestation_id.clone()),
        "PERSISTED",
        "COMPLETED",
        None,
    )
    .await?;
    Ok(())
}

async fn finalize_job_keys(
    infra: &crate::infra::InfraClients,
    request_id: &str,
    attestation_id: &str,
    ttl_seconds: i64,
) -> Result<(), AppError> {
    let ttl_u64 = u64::try_from(ttl_seconds)
        .map_err(|_| AppError::internal("REDIS_ERROR", "invalid ttl for redis"))?;
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connect failed: {e}")))?;
    let idem_key = format!("{IDEMPOTENCY_PREFIX}{request_id}");
    let job_key = format!("{JOB_STATUS_PREFIX}{request_id}");
    let _: () = conn
        .set_ex(idem_key, attestation_id, ttl_u64)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("set idempotency failed: {e}")))?;
    let _: () = conn
        .set_ex(job_key, "COMPLETED", ttl_u64)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("set job status failed: {e}")))?;
    Ok(())
}

async fn write_audit_event(
    infra: &crate::infra::InfraClients,
    request_id: &str,
    attestation_id: Option<String>,
    event_type: &str,
    status: &str,
    details: Option<String>,
) -> Result<(), AppError> {
    let audits: Collection<AuditEventRecord> = infra.mongo_db.collection(AUDIT_EVENTS_COLLECTION);
    audits
        .insert_one(AuditEventRecord {
            request_id: request_id.to_string(),
            attestation_id,
            event_type: event_type.to_string(),
            status: status.to_string(),
            timestamp: now_unix()?,
            details,
        })
        .await
        .map_err(|e| {
            AppError::internal("PERSISTENCE_ERROR", format!("insert audit failed: {e}"))
        })?;
    Ok(())
}

fn decision_from_str(value: &str) -> ComplianceDecision {
    match value {
        "PASS" => ComplianceDecision::Pass,
        "REVIEW" => ComplianceDecision::Review,
        _ => ComplianceDecision::Fail,
    }
}

#[derive(Serialize, Deserialize)]
struct CachedScreening {
    hits: Vec<ScreeningHit>,
    match_digest: String,
}
