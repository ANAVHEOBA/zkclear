use super::error::AppError;
use super::model::{ActivePolicyRecord, AuditLogRecord, PolicySnapshotRecord, RunPolicyEvidenceRecord};
use super::schema::{
    ActivatePolicyRequest, ActivatePolicyResponse, ActivePolicyResponse, CreateSnapshotRequest,
    CreateSnapshotResponse, EffectivePolicyResponse, SnapshotLookupResponse,
};
use crate::app::AppState;
use crate::config::environment::AppConfig;
use crate::infra::{
    ACTIVATION_HISTORY_COLLECTION, AUDIT_LOG_COLLECTION, InfraClients, RUN_EVIDENCE_COLLECTION,
    SNAPSHOTS_COLLECTION,
};
use crate::service::canonical_json_service::{canonical_string, canonicalize};
use crate::service::hash_service::{hmac_sha256_hex, sha256_hex};
use crate::service::internal_auth_service::verify_signature;
use crate::service::rules_validation_service::validate_rules_bundle;
use axum::http::HeaderMap;
use mongodb::Collection;
use mongodb::bson::doc;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Default)]
pub struct PolicyStore {
    inner: Mutex<PolicyStoreInner>,
}

#[derive(Debug, Default)]
struct PolicyStoreInner {
    snapshots_by_version: HashMap<String, PolicySnapshotRecord>,
    version_by_hash: HashMap<String, String>,
    activation_history: Vec<ActivePolicyRecord>,
    active_idx: Option<usize>,
    run_evidence_by_run: HashMap<String, RunPolicyEvidenceRecord>,
}

pub async fn create_snapshot(
    state: &AppState,
    headers: HeaderMap,
    req: CreateSnapshotRequest,
) -> Result<CreateSnapshotResponse, AppError> {
    maybe_verify_internal_auth(&state.config, &headers, &req)?;
    let resp = create_snapshot_in_memory(&state.store, req.clone())?;

    if let Some(infra) = &state.infra {
        let snapshot = snapshot_by_version(&state.store, &resp.policy_version)?
            .ok_or_else(|| AppError::internal("SNAPSHOT_NOT_FOUND", "snapshot missing after create"))?;
        persist_snapshot(infra, &snapshot).await?;
        cache_snapshot(infra, &snapshot).await?;
        append_audit(
            infra,
            AuditLogRecord {
                event_type: "SNAPSHOT_CREATED".to_string(),
                policy_version: Some(snapshot.policy_version.clone()),
                policy_hash: Some(snapshot.policy_hash.clone()),
                run_id: None,
                timestamp: now_unix()?,
                details: None,
            },
        )
        .await?;
    }

    Ok(resp)
}

pub async fn activate_policy(
    state: &AppState,
    headers: HeaderMap,
    req: ActivatePolicyRequest,
) -> Result<ActivatePolicyResponse, AppError> {
    maybe_verify_internal_auth(&state.config, &headers, &req)?;
    let resp = activate_policy_in_memory(&state.store, req)?;

    if let Some(infra) = &state.infra {
        persist_activation(infra, &resp.active_mapping).await?;
        cache_active_mapping(infra, &resp.active_mapping).await?;
        append_audit(
            infra,
            AuditLogRecord {
                event_type: "POLICY_ACTIVATED".to_string(),
                policy_version: Some(resp.active_mapping.policy_version.clone()),
                policy_hash: Some(resp.active_mapping.policy_hash.clone()),
                run_id: None,
                timestamp: now_unix()?,
                details: Some(format!(
                    "onchain_policy_version={}",
                    resp.active_mapping.onchain_policy_version
                )),
            },
        )
        .await?;
    }

    Ok(resp)
}

pub async fn get_snapshot_by_version(
    state: &AppState,
    policy_version: &str,
) -> Result<SnapshotLookupResponse, AppError> {
    let mut resp = get_snapshot_by_version_in_memory(&state.store, policy_version)?;
    if resp.found {
        return Ok(resp);
    }

    let Some(infra) = &state.infra else {
        return Ok(resp);
    };

    if let Some(snapshot) = read_snapshot_by_version(infra, policy_version).await? {
        warm_snapshot_in_memory(&state.store, &snapshot)?;
        resp = SnapshotLookupResponse {
            found: true,
            snapshot: Some(snapshot),
            active_mapping: get_current_active(&state.store)?,
            error_code: None,
            reason: "snapshot found".to_string(),
        };
    }
    Ok(resp)
}

pub async fn get_snapshot_by_hash(
    state: &AppState,
    policy_hash: &str,
) -> Result<SnapshotLookupResponse, AppError> {
    let mut resp = get_snapshot_by_hash_in_memory(&state.store, policy_hash)?;
    if resp.found {
        return Ok(resp);
    }

    let Some(infra) = &state.infra else {
        return Ok(resp);
    };

    if let Some(snapshot) = read_snapshot_by_hash(infra, policy_hash).await? {
        warm_snapshot_in_memory(&state.store, &snapshot)?;
        resp = SnapshotLookupResponse {
            found: true,
            snapshot: Some(snapshot),
            active_mapping: get_current_active(&state.store)?,
            error_code: None,
            reason: "snapshot found".to_string(),
        };
    }
    Ok(resp)
}

pub async fn get_active_policy(state: &AppState) -> Result<ActivePolicyResponse, AppError> {
    let mut resp = get_active_policy_in_memory(&state.store)?;
    if resp.found {
        return Ok(resp);
    }

    let Some(infra) = &state.infra else {
        return Ok(resp);
    };

    if let Some(active) = read_active_mapping(infra).await? {
        warm_activation_in_memory(&state.store, &active)?;
        let snapshot = get_snapshot_by_version(state, &active.policy_version)
            .await?
            .snapshot;
        resp = ActivePolicyResponse {
            found: snapshot.is_some(),
            active_mapping: Some(active),
            snapshot,
            error_code: None,
            reason: "active policy found".to_string(),
        };
    }
    Ok(resp)
}

pub async fn get_active_policy_at_timestamp(
    state: &AppState,
    timestamp: i64,
) -> Result<ActivePolicyResponse, AppError> {
    let mut resp = get_active_policy_at_timestamp_in_memory(&state.store, timestamp)?;
    if resp.found {
        return Ok(resp);
    }

    let Some(infra) = &state.infra else {
        return Ok(resp);
    };
    if let Some(active) = read_active_mapping_at_timestamp(infra, timestamp).await? {
        warm_activation_in_memory(&state.store, &active)?;
        let snapshot = get_snapshot_by_version(state, &active.policy_version)
            .await?
            .snapshot;
        resp = ActivePolicyResponse {
            found: snapshot.is_some(),
            active_mapping: Some(active),
            snapshot,
            error_code: None,
            reason: "active policy found".to_string(),
        };
    }
    Ok(resp)
}

pub async fn get_effective_policy_for_run(
    config: &AppConfig,
    store: &PolicyStore,
    infra: Option<&InfraClients>,
    run_id: &str,
    run_timestamp: i64,
    version_hint: Option<String>,
) -> Result<EffectivePolicyResponse, AppError> {
    if run_id.trim().is_empty() {
        return Err(AppError::bad_request("INVALID_RUN_ID", "run_id is required"));
    }
    if run_timestamp <= 0 {
        return Err(AppError::bad_request(
            "INVALID_RUN_TIMESTAMP",
            "timestamp must be positive unix epoch",
        ));
    }

    if let Some(existing) = run_evidence_in_memory(store, run_id)? {
        let same = existing.run_timestamp == run_timestamp && existing.version_hint == version_hint;
        if !same {
            return Err(AppError::conflict(
                "RUN_EVIDENCE_CONFLICT",
                "run_id already resolved with different metadata",
            ));
        }
        let snapshot = snapshot_by_version(store, &existing.policy_version)?;
        return Ok(EffectivePolicyResponse {
            found: snapshot.is_some(),
            run_id: run_id.to_string(),
            snapshot,
            activation: Some(active_record_from_evidence(&existing)),
            evidence: Some(existing),
            error_code: None,
            reason: "effective policy found".to_string(),
        });
    }

    if let Some(infra) = infra {
        if let Some(existing) = read_run_evidence(infra, run_id).await? {
            let same = existing.run_timestamp == run_timestamp && existing.version_hint == version_hint;
            if !same {
                return Err(AppError::conflict(
                    "RUN_EVIDENCE_CONFLICT",
                    "run_id already resolved with different metadata",
                ));
            }
            warm_run_evidence_in_memory(store, &existing)?;
            let snapshot = snapshot_by_version(store, &existing.policy_version)?;
            return Ok(EffectivePolicyResponse {
                found: snapshot.is_some(),
                run_id: run_id.to_string(),
                snapshot,
                activation: Some(active_record_from_evidence(&existing)),
                evidence: Some(existing),
                error_code: None,
                reason: "effective policy found".to_string(),
            });
        }
    }

    let activation = match &version_hint {
        Some(version) => active_for_version_at_timestamp(store, version, run_timestamp)?,
        None => active_at_timestamp(store, run_timestamp)?
            .ok_or_else(|| AppError::not_found("ACTIVE_POLICY_NOT_FOUND_AT_TIMESTAMP", "no active policy"))?,
    };

    let snapshot = snapshot_by_version(store, &activation.policy_version)?
        .ok_or_else(|| AppError::internal("SNAPSHOT_MISSING", "active mapping references missing snapshot"))?;

    let evidence_hash = compute_run_evidence_hash(
        run_id,
        run_timestamp,
        version_hint.as_deref(),
        &snapshot.policy_version,
        &snapshot.policy_hash,
        activation.activated_at,
        activation.deactivated_at,
    )?;
    let evidence_signature = match &config.audit_signing_secret {
        Some(secret) => Some(
            hmac_sha256_hex(&evidence_hash, secret)
                .map_err(|e| AppError::internal("AUDIT_SIGNATURE_ERROR", e))?,
        ),
        None => None,
    };
    let evidence = RunPolicyEvidenceRecord {
        run_id: run_id.to_string(),
        run_timestamp,
        version_hint: version_hint.clone(),
        policy_version: snapshot.policy_version.clone(),
        policy_hash: snapshot.policy_hash.clone(),
        activated_at: activation.activated_at,
        deactivated_at: activation.deactivated_at,
        evidence_hash,
        evidence_signature,
        created_at: now_unix()?,
    };
    warm_run_evidence_in_memory(store, &evidence)?;

    if let Some(infra) = infra {
        persist_run_evidence(infra, &evidence).await?;
        append_audit(
            infra,
            AuditLogRecord {
                event_type: "RUN_EFFECTIVE_POLICY_RESOLVED".to_string(),
                policy_version: Some(evidence.policy_version.clone()),
                policy_hash: Some(evidence.policy_hash.clone()),
                run_id: Some(run_id.to_string()),
                timestamp: now_unix()?,
                details: None,
            },
        )
        .await?;
    }

    Ok(EffectivePolicyResponse {
        found: true,
        run_id: run_id.to_string(),
        snapshot: Some(snapshot),
        activation: Some(activation),
        evidence: Some(evidence),
        error_code: None,
        reason: "effective policy resolved".to_string(),
    })
}

fn create_snapshot_in_memory(
    store: &PolicyStore,
    req: CreateSnapshotRequest,
) -> Result<CreateSnapshotResponse, AppError> {
    if req.policy_version.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_POLICY_VERSION",
            "policy_version is required",
        ));
    }
    validate_rules_bundle(&req.rules).map_err(|e| AppError::bad_request("INVALID_RULES_BUNDLE", e))?;

    let canonical_rules = canonicalize(&req.rules);
    let canonical_rules_json = canonical_string(&canonical_rules)?;
    let computed_hash = sha256_hex(&canonical_rules_json);
    if let Some(provided) = &req.policy_hash {
        if provided != &computed_hash {
            return Err(AppError::bad_request(
                "POLICY_HASH_MISMATCH",
                "provided policy_hash does not match canonical rule bundle hash",
            ));
        }
    }

    let mut inner = lock_store(store)?;
    if let Some(existing) = inner.snapshots_by_version.get(&req.policy_version) {
        if existing.policy_hash == computed_hash && existing.canonical_rules_json == canonical_rules_json {
            return Ok(CreateSnapshotResponse {
                accepted: true,
                idempotent: true,
                policy_version: existing.policy_version.clone(),
                policy_hash: existing.policy_hash.clone(),
                canonical_rules_json: existing.canonical_rules_json.clone(),
                error_code: None,
                reason: "snapshot already exists".to_string(),
            });
        }
        return Err(AppError::conflict(
            "IMMUTABLE_VERSION_CONFLICT",
            "policy_version already exists with different payload",
        ));
    }
    if let Some(existing_version) = inner.version_by_hash.get(&computed_hash) {
        return Err(AppError::conflict(
            "IMMUTABLE_HASH_CONFLICT",
            format!("policy_hash already registered under version {existing_version}"),
        ));
    }

    let record = PolicySnapshotRecord {
        policy_version: req.policy_version.clone(),
        policy_hash: computed_hash.clone(),
        canonical_rules,
        canonical_rules_json: canonical_rules_json.clone(),
        metadata: req.metadata,
        created_at: now_unix()?,
    };
    inner.snapshots_by_version.insert(req.policy_version.clone(), record);
    inner.version_by_hash.insert(computed_hash.clone(), req.policy_version.clone());

    Ok(CreateSnapshotResponse {
        accepted: true,
        idempotent: false,
        policy_version: req.policy_version,
        policy_hash: computed_hash,
        canonical_rules_json,
        error_code: None,
        reason: "snapshot created".to_string(),
    })
}

fn activate_policy_in_memory(
    store: &PolicyStore,
    req: ActivatePolicyRequest,
) -> Result<ActivatePolicyResponse, AppError> {
    if req.onchain_policy_version.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_ONCHAIN_POLICY_VERSION",
            "onchain_policy_version is required",
        ));
    }
    if req.policy_version.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_POLICY_VERSION",
            "policy_version is required",
        ));
    }

    let mut inner = lock_store(store)?;
    let snapshot = inner
        .snapshots_by_version
        .get(&req.policy_version)
        .ok_or_else(|| AppError::not_found("POLICY_VERSION_NOT_FOUND", "cannot activate unknown policy"))?
        .clone();

    let now = now_unix()?;
    if let Some(idx) = inner.active_idx {
        if let Some(prev) = inner.activation_history.get_mut(idx) {
            prev.deactivated_at = Some(now);
        }
    }
    let active = ActivePolicyRecord {
        onchain_policy_version: req.onchain_policy_version,
        policy_version: snapshot.policy_version,
        policy_hash: snapshot.policy_hash,
        activated_at: now,
        deactivated_at: None,
    };
    inner.activation_history.push(active.clone());
    inner.active_idx = Some(inner.activation_history.len() - 1);

    Ok(ActivatePolicyResponse {
        accepted: true,
        active_mapping: active,
        error_code: None,
        reason: "policy activated".to_string(),
    })
}

fn get_snapshot_by_version_in_memory(
    store: &PolicyStore,
    policy_version: &str,
) -> Result<SnapshotLookupResponse, AppError> {
    let inner = lock_store(store)?;
    let snapshot = inner.snapshots_by_version.get(policy_version).cloned();
    Ok(match snapshot {
        Some(s) => SnapshotLookupResponse {
            found: true,
            snapshot: Some(s),
            active_mapping: current_active(&inner),
            error_code: None,
            reason: "snapshot found".to_string(),
        },
        None => SnapshotLookupResponse {
            found: false,
            snapshot: None,
            active_mapping: current_active(&inner),
            error_code: Some("POLICY_VERSION_NOT_FOUND".to_string()),
            reason: "snapshot not found".to_string(),
        },
    })
}

fn get_snapshot_by_hash_in_memory(
    store: &PolicyStore,
    policy_hash: &str,
) -> Result<SnapshotLookupResponse, AppError> {
    let inner = lock_store(store)?;
    let Some(version) = inner.version_by_hash.get(policy_hash).cloned() else {
        return Ok(SnapshotLookupResponse {
            found: false,
            snapshot: None,
            active_mapping: current_active(&inner),
            error_code: Some("POLICY_HASH_NOT_FOUND".to_string()),
            reason: "snapshot not found".to_string(),
        });
    };
    let snapshot = inner.snapshots_by_version.get(&version).cloned();
    Ok(SnapshotLookupResponse {
        found: snapshot.is_some(),
        snapshot,
        active_mapping: current_active(&inner),
        error_code: None,
        reason: "snapshot found".to_string(),
    })
}

fn get_active_policy_in_memory(store: &PolicyStore) -> Result<ActivePolicyResponse, AppError> {
    let inner = lock_store(store)?;
    let Some(active) = current_active(&inner) else {
        return Ok(ActivePolicyResponse {
            found: false,
            active_mapping: None,
            snapshot: None,
            error_code: Some("ACTIVE_POLICY_NOT_SET".to_string()),
            reason: "no active policy".to_string(),
        });
    };
    let snapshot = inner.snapshots_by_version.get(&active.policy_version).cloned();
    Ok(ActivePolicyResponse {
        found: snapshot.is_some(),
        active_mapping: Some(active),
        snapshot,
        error_code: None,
        reason: "active policy found".to_string(),
    })
}

fn get_active_policy_at_timestamp_in_memory(
    store: &PolicyStore,
    timestamp: i64,
) -> Result<ActivePolicyResponse, AppError> {
    let inner = lock_store(store)?;
    let Some(active) = active_at_timestamp_locked(&inner, timestamp) else {
        return Ok(ActivePolicyResponse {
            found: false,
            active_mapping: None,
            snapshot: None,
            error_code: Some("ACTIVE_POLICY_NOT_FOUND_AT_TIMESTAMP".to_string()),
            reason: "no active policy at timestamp".to_string(),
        });
    };
    let snapshot = inner.snapshots_by_version.get(&active.policy_version).cloned();
    Ok(ActivePolicyResponse {
        found: snapshot.is_some(),
        active_mapping: Some(active),
        snapshot,
        error_code: None,
        reason: "active policy found".to_string(),
    })
}

fn snapshot_by_version(
    store: &PolicyStore,
    policy_version: &str,
) -> Result<Option<PolicySnapshotRecord>, AppError> {
    let inner = lock_store(store)?;
    Ok(inner.snapshots_by_version.get(policy_version).cloned())
}

fn get_current_active(store: &PolicyStore) -> Result<Option<ActivePolicyRecord>, AppError> {
    let inner = lock_store(store)?;
    Ok(current_active(&inner))
}

fn run_evidence_in_memory(
    store: &PolicyStore,
    run_id: &str,
) -> Result<Option<RunPolicyEvidenceRecord>, AppError> {
    let inner = lock_store(store)?;
    Ok(inner.run_evidence_by_run.get(run_id).cloned())
}

fn warm_snapshot_in_memory(store: &PolicyStore, snapshot: &PolicySnapshotRecord) -> Result<(), AppError> {
    let mut inner = lock_store(store)?;
    inner
        .version_by_hash
        .insert(snapshot.policy_hash.clone(), snapshot.policy_version.clone());
    inner
        .snapshots_by_version
        .insert(snapshot.policy_version.clone(), snapshot.clone());
    Ok(())
}

fn warm_activation_in_memory(store: &PolicyStore, active: &ActivePolicyRecord) -> Result<(), AppError> {
    let mut inner = lock_store(store)?;
    inner.activation_history.push(active.clone());
    inner.active_idx = Some(inner.activation_history.len() - 1);
    Ok(())
}

fn warm_run_evidence_in_memory(
    store: &PolicyStore,
    evidence: &RunPolicyEvidenceRecord,
) -> Result<(), AppError> {
    let mut inner = lock_store(store)?;
    inner
        .run_evidence_by_run
        .insert(evidence.run_id.clone(), evidence.clone());
    Ok(())
}

fn active_at_timestamp(
    store: &PolicyStore,
    timestamp: i64,
) -> Result<Option<ActivePolicyRecord>, AppError> {
    let inner = lock_store(store)?;
    Ok(active_at_timestamp_locked(&inner, timestamp))
}

fn active_for_version_at_timestamp(
    store: &PolicyStore,
    version: &str,
    timestamp: i64,
) -> Result<ActivePolicyRecord, AppError> {
    let inner = lock_store(store)?;
    inner
        .activation_history
        .iter()
        .find(|a| {
            a.policy_version == version
                && a.activated_at <= timestamp
                && a.deactivated_at.is_none_or(|d| timestamp < d)
        })
        .cloned()
        .ok_or_else(|| {
            AppError::not_found(
                "POLICY_VERSION_NOT_ACTIVE_AT_TIMESTAMP",
                "version_hint not active at timestamp",
            )
        })
}

fn maybe_verify_internal_auth<T: serde::Serialize>(
    config: &AppConfig,
    headers: &HeaderMap,
    request: &T,
) -> Result<(), AppError> {
    if !config.require_internal_auth {
        return Ok(());
    }
    let secret = config
        .internal_auth_secret
        .as_deref()
        .ok_or_else(|| AppError::internal("AUTH_CONFIG_MISSING", "INTERNAL_AUTH_SECRET is required"))?;
    let signature = headers
        .get("x-internal-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::bad_request("MISSING_SIGNATURE", "x-internal-signature required"))?;
    let payload = serde_json::to_string(request)
        .map_err(|e| AppError::internal("SERIALIZATION_ERROR", format!("auth payload failed: {e}")))?;
    verify_signature(&payload, signature, secret)
        .map_err(|e| AppError::bad_request("BAD_SIGNATURE", e))
}

async fn persist_snapshot(infra: &InfraClients, snapshot: &PolicySnapshotRecord) -> Result<(), AppError> {
    let coll: Collection<PolicySnapshotRecord> = infra.mongo_db.collection(SNAPSHOTS_COLLECTION);
    coll.insert_one(snapshot)
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("insert snapshot failed: {e}")))?;
    Ok(())
}

async fn persist_activation(infra: &InfraClients, active: &ActivePolicyRecord) -> Result<(), AppError> {
    let coll: Collection<ActivePolicyRecord> = infra.mongo_db.collection(ACTIVATION_HISTORY_COLLECTION);
    coll.insert_one(active)
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("insert activation failed: {e}")))?;
    Ok(())
}

async fn persist_run_evidence(
    infra: &InfraClients,
    evidence: &RunPolicyEvidenceRecord,
) -> Result<(), AppError> {
    let coll: Collection<RunPolicyEvidenceRecord> = infra.mongo_db.collection(RUN_EVIDENCE_COLLECTION);
    coll.insert_one(evidence)
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("insert evidence failed: {e}")))?;
    Ok(())
}

async fn append_audit(infra: &InfraClients, record: AuditLogRecord) -> Result<(), AppError> {
    let coll: Collection<AuditLogRecord> = infra.mongo_db.collection(AUDIT_LOG_COLLECTION);
    coll.insert_one(record)
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("insert audit failed: {e}")))?;
    Ok(())
}

async fn cache_snapshot(infra: &InfraClients, snapshot: &PolicySnapshotRecord) -> Result<(), AppError> {
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connect failed: {e}")))?;
    let by_version_key = format!("policy:snapshot:version:{}", snapshot.policy_version);
    let by_hash_key = format!("policy:snapshot:hash:{}", snapshot.policy_hash);
    let payload = serde_json::to_string(snapshot)
        .map_err(|e| AppError::internal("SERIALIZATION_ERROR", format!("snapshot cache encode failed: {e}")))?;
    let _: () = conn
        .set_ex(by_version_key, payload, 3600)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("cache snapshot failed: {e}")))?;
    let _: () = conn
        .set_ex(by_hash_key, snapshot.policy_version.clone(), 3600)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("cache hash map failed: {e}")))?;
    Ok(())
}

async fn cache_active_mapping(infra: &InfraClients, active: &ActivePolicyRecord) -> Result<(), AppError> {
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connect failed: {e}")))?;
    let payload = serde_json::to_string(active)
        .map_err(|e| AppError::internal("SERIALIZATION_ERROR", format!("active cache encode failed: {e}")))?;
    let _: () = conn
        .set_ex("policy:active", payload, 3600)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("cache active failed: {e}")))?;
    Ok(())
}

async fn read_snapshot_by_version(
    infra: &InfraClients,
    policy_version: &str,
) -> Result<Option<PolicySnapshotRecord>, AppError> {
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connect failed: {e}")))?;
    let cache_key = format!("policy:snapshot:version:{policy_version}");
    let cached: Option<String> = conn
        .get(&cache_key)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis read snapshot failed: {e}")))?;
    if let Some(raw) = cached {
        let parsed = serde_json::from_str::<PolicySnapshotRecord>(&raw)
            .map_err(|e| AppError::internal("REDIS_ERROR", format!("cached snapshot parse failed: {e}")))?;
        return Ok(Some(parsed));
    }

    let coll: Collection<PolicySnapshotRecord> = infra.mongo_db.collection(SNAPSHOTS_COLLECTION);
    coll.find_one(doc! { "policy_version": policy_version })
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("read snapshot failed: {e}")))
}

async fn read_snapshot_by_hash(
    infra: &InfraClients,
    policy_hash: &str,
) -> Result<Option<PolicySnapshotRecord>, AppError> {
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connect failed: {e}")))?;
    let hash_key = format!("policy:snapshot:hash:{policy_hash}");
    let mapped_version: Option<String> = conn
        .get(&hash_key)
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis read hash map failed: {e}")))?;
    if let Some(version) = mapped_version {
        return read_snapshot_by_version(infra, &version).await;
    }
    let coll: Collection<PolicySnapshotRecord> = infra.mongo_db.collection(SNAPSHOTS_COLLECTION);
    coll.find_one(doc! { "policy_hash": policy_hash })
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("read snapshot by hash failed: {e}")))
}

async fn read_active_mapping(infra: &InfraClients) -> Result<Option<ActivePolicyRecord>, AppError> {
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connect failed: {e}")))?;
    let cached: Option<String> = conn
        .get("policy:active")
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis read active failed: {e}")))?;
    if let Some(raw) = cached {
        let parsed = serde_json::from_str::<ActivePolicyRecord>(&raw)
            .map_err(|e| AppError::internal("REDIS_ERROR", format!("cached active parse failed: {e}")))?;
        return Ok(Some(parsed));
    }
    let coll: Collection<ActivePolicyRecord> = infra.mongo_db.collection(ACTIVATION_HISTORY_COLLECTION);
    coll.find_one(doc! {})
        .sort(doc! {"activated_at": -1})
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("read active mapping failed: {e}")))
}

async fn read_active_mapping_at_timestamp(
    infra: &InfraClients,
    timestamp: i64,
) -> Result<Option<ActivePolicyRecord>, AppError> {
    let coll: Collection<ActivePolicyRecord> = infra.mongo_db.collection(ACTIVATION_HISTORY_COLLECTION);
    coll.find_one(doc! {
        "activated_at": { "$lte": timestamp },
        "$or": [
            { "deactivated_at": { "$exists": false } },
            { "deactivated_at": null },
            { "deactivated_at": { "$gt": timestamp } }
        ]
    })
    .sort(doc! { "activated_at": -1 })
    .await
    .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("read active at timestamp failed: {e}")))
}

async fn read_run_evidence(
    infra: &InfraClients,
    run_id: &str,
) -> Result<Option<RunPolicyEvidenceRecord>, AppError> {
    let coll: Collection<RunPolicyEvidenceRecord> = infra.mongo_db.collection(RUN_EVIDENCE_COLLECTION);
    coll.find_one(doc! { "run_id": run_id })
        .await
        .map_err(|e| AppError::internal("PERSISTENCE_ERROR", format!("read run evidence failed: {e}")))
}

fn lock_store(store: &PolicyStore) -> Result<MutexGuard<'_, PolicyStoreInner>, AppError> {
    store
        .inner
        .lock()
        .map_err(|_| AppError::internal("STORE_LOCK_ERROR", "policy store lock poisoned"))
}

fn now_unix() -> Result<i64, AppError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal("CLOCK_ERROR", format!("clock error: {e}")))?;
    Ok(elapsed.as_secs() as i64)
}

fn current_active(inner: &PolicyStoreInner) -> Option<ActivePolicyRecord> {
    inner
        .active_idx
        .and_then(|idx| inner.activation_history.get(idx).cloned())
}

fn active_at_timestamp_locked(inner: &PolicyStoreInner, timestamp: i64) -> Option<ActivePolicyRecord> {
    inner
        .activation_history
        .iter()
        .filter(|rec| rec.activated_at <= timestamp && rec.deactivated_at.is_none_or(|d| timestamp < d))
        .max_by_key(|rec| rec.activated_at)
        .cloned()
}

fn compute_run_evidence_hash(
    run_id: &str,
    run_timestamp: i64,
    version_hint: Option<&str>,
    policy_version: &str,
    policy_hash: &str,
    activated_at: i64,
    deactivated_at: Option<i64>,
) -> Result<String, AppError> {
    #[derive(serde::Serialize)]
    struct Payload<'a> {
        run_id: &'a str,
        run_timestamp: i64,
        version_hint: Option<&'a str>,
        policy_version: &'a str,
        policy_hash: &'a str,
        activated_at: i64,
        deactivated_at: Option<i64>,
    }
    let payload = serde_json::to_string(&Payload {
        run_id,
        run_timestamp,
        version_hint,
        policy_version,
        policy_hash,
        activated_at,
        deactivated_at,
    })
    .map_err(|e| AppError::internal("SERIALIZATION_ERROR", format!("evidence payload failed: {e}")))?;
    Ok(sha256_hex(&payload))
}

fn active_record_from_evidence(e: &RunPolicyEvidenceRecord) -> ActivePolicyRecord {
    ActivePolicyRecord {
        onchain_policy_version: String::new(),
        policy_version: e.policy_version.clone(),
        policy_hash: e.policy_hash.clone(),
        activated_at: e.activated_at,
        deactivated_at: e.deactivated_at,
    }
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        AppError::internal("INTERNAL_ERROR", value)
    }
}

impl From<Value> for AppError {
    fn from(_: Value) -> Self {
        AppError::internal("INTERNAL_ERROR", "unexpected value conversion")
    }
}
