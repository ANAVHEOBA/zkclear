use super::crud;
use super::error::AppError;
use super::schema::{
    GetProofJobResponse, GetProofJobsByRunResponse, HealthMetricsView, HealthResponse,
    OtcComplianceSubjectResult, OtcIntentSubmitResult, QueueStatsResponse, RetryProofJobResponse,
    StartOtcOrchestrationRequest, StartOtcOrchestrationResponse, SubmitProofJobRequest,
    SubmitProofJobResponse,
    UpdateProofJobStatusRequest, UpdateProofJobStatusResponse, WalletMeResponse,
    WalletNonceRequest, WalletNonceResponse, WalletVerifyRequest, WalletVerifyResponse,
};
use crate::app::AppState;
use crate::service::internal_auth_service::verify_internal_signature;
use crate::service::metrics_service;
use crate::service::queue_service;
use crate::service::wallet_auth_service::{
    WalletClaims, build_login_message, issue_access_token, normalize_wallet_address,
    resolve_wallet_role, verify_access_token, verify_personal_sign,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use serde_json::json;
use tracing::{error, info};
use uuid::Uuid;

pub async fn wallet_nonce(
    State(state): State<AppState>,
    Json(req): Json<WalletNonceRequest>,
) -> impl IntoResponse {
    if !state.config.wallet_auth_enabled {
        return error_wallet_nonce(AppError::bad_request(
            "WALLET_AUTH_DISABLED",
            "wallet auth is disabled",
        ));
    }
    let wallet = match normalize_wallet_address(&req.wallet_address) {
        Ok(v) => v,
        Err(e) => return error_wallet_nonce(AppError::bad_request("INVALID_WALLET", e)),
    };
    if state.config.wallet_auth_nonce_ttl_seconds <= 0 {
        return error_wallet_nonce(AppError::internal(
            "AUTH_CONFIG_ERROR",
            "WALLET_AUTH_NONCE_TTL_SECONDS must be positive",
        ));
    }

    let now = chrono::Utc::now().timestamp();
    let expires_at = now
        .checked_add(state.config.wallet_auth_nonce_ttl_seconds)
        .unwrap_or(now);
    let nonce = Uuid::new_v4().simple().to_string();
    let message = build_login_message(&wallet, &nonce, now, state.config.eth_sepolia_chain_id);

    {
        let mut guard = state.wallet_nonces.write().await;
        guard.retain(|_, challenge| challenge.expires_at >= now);
        guard.insert(
            wallet.clone(),
            crate::app::WalletNonceChallenge {
                nonce: nonce.clone(),
                message: message.clone(),
                expires_at,
            },
        );
    }

    (
        axum::http::StatusCode::OK,
        Json(WalletNonceResponse {
            accepted: true,
            wallet_address: wallet,
            nonce,
            message,
            expires_at,
            error_code: None,
            reason: "nonce issued".to_string(),
        }),
    )
}

pub async fn wallet_verify(
    State(state): State<AppState>,
    Json(req): Json<WalletVerifyRequest>,
) -> impl IntoResponse {
    if !state.config.wallet_auth_enabled {
        return error_wallet_verify(AppError::bad_request(
            "WALLET_AUTH_DISABLED",
            "wallet auth is disabled",
        ));
    }
    let wallet = match normalize_wallet_address(&req.wallet_address) {
        Ok(v) => v,
        Err(e) => return error_wallet_verify(AppError::bad_request("INVALID_WALLET", e)),
    };

    let challenge = {
        let mut guard = state.wallet_nonces.write().await;
        let now = chrono::Utc::now().timestamp();
        guard.retain(|_, c| c.expires_at >= now);
        guard.remove(&wallet)
    };
    let challenge = match challenge {
        Some(v) => v,
        None => {
            return error_wallet_verify(AppError::bad_request(
                "NONCE_NOT_FOUND",
                "wallet nonce not found or expired",
            ));
        }
    };

    let recovered = match verify_personal_sign(&challenge.message, &req.signature) {
        Ok(v) => v,
        Err(e) => return error_wallet_verify(AppError::unauthorized("BAD_SIGNATURE", e)),
    };
    if recovered != wallet {
        return error_wallet_verify(AppError::unauthorized(
            "SIGNER_MISMATCH",
            "signature signer does not match wallet_address",
        ));
    }
    let role = resolve_wallet_role(
        &wallet,
        &state.config.wallet_role_map,
        &state.config.wallet_default_role,
    );
    let jwt_secret = match state.config.wallet_jwt_secret.as_deref() {
        Some(v) if !v.trim().is_empty() => v,
        _ => {
            return error_wallet_verify(AppError::internal(
                "AUTH_CONFIG_ERROR",
                "WALLET_JWT_SECRET is required for wallet auth",
            ));
        }
    };
    let (token, expires_at) = match issue_access_token(
        &wallet,
        &role,
        jwt_secret,
        state.config.wallet_jwt_ttl_seconds,
    ) {
        Ok(v) => v,
        Err(e) => return error_wallet_verify(AppError::internal("TOKEN_ISSUE_ERROR", e)),
    };
    info!(
        wallet_address = %wallet,
        role = %role,
        nonce = %challenge.nonce,
        "wallet login successful"
    );
    (
        axum::http::StatusCode::OK,
        Json(WalletVerifyResponse {
            accepted: true,
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_at,
            wallet_address: wallet,
            role,
            error_code: None,
            reason: "wallet authenticated".to_string(),
        }),
    )
}

pub async fn wallet_me(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !state.config.wallet_auth_enabled {
        return (
            axum::http::StatusCode::OK,
            Json(WalletMeResponse {
                authenticated: false,
                wallet_address: String::new(),
                role: String::new(),
                error_code: Some("WALLET_AUTH_DISABLED".to_string()),
                reason: "wallet auth is disabled".to_string(),
            }),
        );
    }
    match verify_wallet_bearer_claims(&state, &headers) {
        Ok(claims) => (
            axum::http::StatusCode::OK,
            Json(WalletMeResponse {
                authenticated: true,
                wallet_address: claims.sub,
                role: claims.role,
                error_code: None,
                reason: "authenticated".to_string(),
            }),
        ),
        Err(err) => (
            err.status,
            Json(WalletMeResponse {
                authenticated: false,
                wallet_address: String::new(),
                role: String::new(),
                error_code: Some(err.code.to_string()),
                reason: err.message,
            }),
        ),
    }
}

pub async fn submit_proof_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SubmitProofJobRequest>,
) -> impl IntoResponse {
    if let Err(err) = verify_write_auth(&state, &headers, &req) {
        return error_submit(err);
    }

    match crud::submit_proof_job(&state, req).await {
        Ok(resp) => {
            info!(job_id = %resp.job_id, workflow_run_id = %resp.workflow_run_id, proof_type = %resp.proof_type, "proof job accepted");
            (axum::http::StatusCode::OK, Json(resp))
        }
        Err(err) => error_submit(err),
    }
}

pub async fn start_otc_orchestration(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<StartOtcOrchestrationRequest>,
) -> impl IntoResponse {
    if let Err(err) = verify_write_auth(&state, &headers, &req) {
        return error_orchestration(err);
    }
    if req.intents.len() != 2 {
        return error_orchestration(AppError::bad_request(
            "INVALID_INTENT_COUNT",
            "exactly two intents are required",
        ));
    }
    if req.subjects.is_empty() {
        return error_orchestration(AppError::bad_request(
            "INVALID_SUBJECTS",
            "at least one compliance subject is required",
        ));
    }

    let workflow_run_id = format!("run-{}", Uuid::new_v4());
    let client = reqwest::Client::new();
    let mut intent_submissions = Vec::with_capacity(2);

    for intent in &req.intents {
        let response = match client
            .post(format!(
                "{}/v1/intents/submit",
                state.config.intent_gateway_base_url.trim_end_matches('/')
            ))
            .header("content-type", "application/json")
            .header("x-workflow-run-id", &workflow_run_id)
            .json(intent)
            .send()
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return error_orchestration(AppError::internal(
                    "INTENT_GATEWAY_UNAVAILABLE",
                    format!("intent submit failed: {e}"),
                ));
            }
        };
        let status = response.status();
        let raw = match response.text().await {
            Ok(v) => v,
            Err(e) => {
                return error_orchestration(AppError::internal(
                    "INTENT_GATEWAY_DECODE_ERROR",
                    format!("failed reading intent gateway body: {e}"),
                ));
            }
        };
        let payload = match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => v,
            Err(e) => {
                return error_orchestration(AppError::internal(
                    "INTENT_GATEWAY_DECODE_ERROR",
                    format!(
                        "non-json from intent gateway status={} body={} parse_err={}",
                        status.as_u16(),
                        raw,
                        e
                    ),
                ));
            }
        };

        let submit = OtcIntentSubmitResult {
            accepted: payload
                .get("accepted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            workflow_run_id: payload
                .get("workflow_run_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            intent_ids: payload
                .get("intent_ids")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            commitment_hashes: payload
                .get("commitment_hashes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            error_code: payload
                .get("error_code")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned),
            reason: payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        };

        if !status.is_success() || !submit.accepted {
            return error_orchestration(AppError::internal(
                "INTENT_SUBMIT_REJECTED",
                format!(
                    "intent submit rejected: status={} reason={} code={}",
                    status.as_u16(),
                    submit.reason,
                    submit.error_code.clone().unwrap_or_default()
                ),
            ));
        }

        intent_submissions.push(submit);
    }

    let policy_resp = match client
        .get(format!(
            "{}/v1/policy/active",
            state.config.policy_snapshot_base_url.trim_end_matches('/')
        ))
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return error_orchestration(AppError::internal(
                "POLICY_SERVICE_UNAVAILABLE",
                format!("policy lookup failed: {e}"),
            ));
        }
    };
    let policy_json = match policy_resp.json::<serde_json::Value>().await {
        Ok(v) => v,
        Err(e) => {
            return error_orchestration(AppError::internal(
                "POLICY_SERVICE_DECODE_ERROR",
                e.to_string(),
            ));
        }
    };
    let policy_version = policy_json
        .get("active_mapping")
        .and_then(|v| v.get("policy_version"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let policy_hash = policy_json
        .get("active_mapping")
        .and_then(|v| v.get("policy_hash"))
        .or_else(|| policy_json.get("snapshot").and_then(|v| v.get("policy_hash")))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if policy_version.is_empty() {
        return error_orchestration(AppError::internal(
            "POLICY_NOT_ACTIVE",
            "no active policy returned",
        ));
    }

    let compliance_req = json!({
        "workflow_run_id": workflow_run_id,
        "request_id": req.request_id.unwrap_or_else(|| format!("req-{}", Uuid::new_v4())),
        "nonce": req.compliance_nonce.unwrap_or_else(|| format!("nonce-{}", Uuid::new_v4())),
        "timestamp": chrono::Utc::now().timestamp(),
        "subjects": req.subjects
    });
    let compliance_resp = match client
        .post(format!(
            "{}/v1/compliance/intake",
            state
                .config
                .compliance_adapter_base_url
                .trim_end_matches('/')
        ))
        .header("content-type", "application/json")
        .json(&compliance_req)
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return error_orchestration(AppError::internal(
                "COMPLIANCE_SERVICE_UNAVAILABLE",
                format!("compliance intake failed: {e}"),
            ));
        }
    };
    let compliance_json = match compliance_resp.json::<serde_json::Value>().await {
        Ok(v) => v,
        Err(e) => {
            return error_orchestration(AppError::internal(
                "COMPLIANCE_SERVICE_DECODE_ERROR",
                e.to_string(),
            ));
        }
    };
    let compliance_accepted = compliance_json
        .get("accepted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let compliance_decision = compliance_json
        .get("decision")
        .and_then(|v| v.as_str())
        .unwrap_or("FAIL")
        .to_string();
    let compliance_reason = compliance_json
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("compliance rejected")
        .to_string();
    let compliance_error_code = compliance_json
        .get("error_code")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let compliance_results = build_compliance_results(
        &req.subjects,
        compliance_json.get("normalized_subjects"),
        &compliance_decision,
        compliance_error_code.as_deref(),
    );
    if !compliance_accepted || !compliance_decision.eq_ignore_ascii_case("PASS") {
        return (
            axum::http::StatusCode::OK,
            Json(StartOtcOrchestrationResponse {
                accepted: false,
                workflow_run_id,
                policy_version: policy_version.clone(),
                policy_hash: policy_hash.clone(),
                attestation_id: compliance_json
                    .get("attestation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                attestation_hash: compliance_json
                    .get("attestation_hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                intent_submissions,
                compliance_results,
                proof_job: None,
                error_code: Some(
                    compliance_error_code.unwrap_or_else(|| "COMPLIANCE_DECISION_BLOCK".to_string()),
                ),
                reason: compliance_reason,
            }),
        );
    }

    let attestation_id = compliance_json
        .get("attestation_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let attestation_hash = compliance_json
        .get("attestation_hash")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if attestation_hash.is_empty() {
        return error_orchestration(AppError::internal(
            "COMPLIANCE_ATTESTATION_MISSING",
            "attestation_hash missing from compliance response",
        ));
    }

    let mut receipt_context = req.receipt_context.unwrap_or_else(|| json!({}));
    if !receipt_context.is_object() {
        receipt_context = json!({});
    }
    if receipt_context.get("receiptHash").is_none() && receipt_context.get("receipt_hash").is_none()
    {
        receipt_context["receiptHash"] = json!(attestation_hash.clone());
    }
    if receipt_context.get("binding").is_none() {
        receipt_context["binding"] = json!({
            "workflowRunId": workflow_run_id,
            "policyVersion": policy_version,
            "receiptHash": attestation_hash,
            "domainSeparator": state.config.signal_domain_separator
        });
    }

    let proof_req = SubmitProofJobRequest {
        workflow_run_id: workflow_run_id.clone(),
        policy_version: policy_version.clone(),
        receipt_context,
        proof_type: req.proof_type,
        idempotency_key: req
            .idempotency_key
            .unwrap_or_else(|| format!("idem-{}", Uuid::new_v4())),
    };
    let proof_job = match crud::submit_proof_job(&state, proof_req).await {
        Ok(v) => v,
        Err(err) => return error_orchestration(err),
    };

    (
        axum::http::StatusCode::OK,
        Json(StartOtcOrchestrationResponse {
            accepted: true,
            workflow_run_id,
            policy_version,
            policy_hash,
            attestation_id,
            attestation_hash,
            intent_submissions,
            compliance_results,
            proof_job: Some(proof_job),
            error_code: None,
            reason: "otc orchestration started".to_string(),
        }),
    )
}

pub async fn get_proof_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    match crud::get_proof_job(&state, &job_id).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(err) => error_get(err),
    }
}

pub async fn get_proof_jobs_by_run(
    State(state): State<AppState>,
    Path(workflow_run_id): Path<String>,
) -> impl IntoResponse {
    match crud::get_proof_jobs_by_run(&state, &workflow_run_id).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(AppError {
            status,
            code,
            message,
        }) => {
            error!(error_code = code, reason = %message, workflow_run_id = %workflow_run_id, "proof jobs by run lookup failed");
            (
                status,
                Json(GetProofJobsByRunResponse {
                    found: false,
                    workflow_run_id,
                    jobs: Vec::new(),
                    error_code: Some(code.to_string()),
                    reason: message,
                }),
            )
        }
    }
}

pub async fn retry_proof_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let auth_payload = json!({ "job_id": job_id });
    if let Err(err) = verify_write_auth(&state, &headers, &auth_payload) {
        return error_retry(err);
    }

    match crud::retry_proof_job(&state, &job_id).await {
        Ok(resp) => {
            info!(job_id = %job_id, "proof job manually requeued");
            (axum::http::StatusCode::OK, Json(resp))
        }
        Err(err) => error_retry(err),
    }
}

pub async fn update_proof_job_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Json(req): Json<UpdateProofJobStatusRequest>,
) -> impl IntoResponse {
    if let Err(err) = verify_write_auth(&state, &headers, &req) {
        return error_update(err);
    }

    match crud::update_proof_job_status(&state, &job_id, req).await {
        Ok(resp) => {
            info!(job_id = %job_id, "proof job status updated");
            (axum::http::StatusCode::OK, Json(resp))
        }
        Err(err) => error_update(err),
    }
}

pub async fn get_queue_stats(State(state): State<AppState>) -> impl IntoResponse {
    match queue_service::queue_stats(&state).await {
        Ok((queued, processing, retry_scheduled, dead_letter)) => (
            axum::http::StatusCode::OK,
            Json(QueueStatsResponse {
                available: true,
                queued,
                processing,
                retry_scheduled,
                dead_letter,
                error_code: None,
                reason: "queue stats available".to_string(),
            }),
        ),
        Err(message) => (
            axum::http::StatusCode::OK,
            Json(QueueStatsResponse {
                available: false,
                queued: 0,
                processing: 0,
                retry_scheduled: 0,
                dead_letter: 0,
                error_code: Some("QUEUE_UNAVAILABLE".to_string()),
                reason: message,
            }),
        ),
    }
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let queue = match queue_service::queue_stats(&state).await {
        Ok((queued, processing, retry_scheduled, dead_letter)) => QueueStatsResponse {
            available: true,
            queued,
            processing,
            retry_scheduled,
            dead_letter,
            error_code: None,
            reason: "queue stats available".to_string(),
        },
        Err(message) => QueueStatsResponse {
            available: false,
            queued: 0,
            processing: 0,
            retry_scheduled: 0,
            dead_letter: 0,
            error_code: Some("QUEUE_UNAVAILABLE".to_string()),
            reason: message,
        },
    };

    let m = metrics_service::snapshot();
    let metrics = HealthMetricsView {
        jobs_queued: m.jobs_queued,
        jobs_published: m.jobs_published,
        jobs_failed: m.jobs_failed,
        retries_scheduled: m.retries_scheduled,
        prove_duration_count: m.prove_duration_count,
        prove_duration_avg_ms: m.prove_duration_avg_ms,
        queue_latency_count: m.queue_latency_count,
        queue_latency_avg_ms: m.queue_latency_avg_ms,
        last_error_ts: m.last_error_ts,
    };
    let mongo_available = state.infra.is_some();
    let redis_available = state.infra.is_some() && queue.available;
    let ok = !state.config.worker_enabled || redis_available;

    (
        axum::http::StatusCode::OK,
        Json(HealthResponse {
            ok,
            redis_available,
            mongo_available,
            worker_enabled: state.config.worker_enabled,
            queue,
            metrics,
            error_code: None,
            reason: if ok {
                "healthy".to_string()
            } else {
                "worker enabled but redis unavailable".to_string()
            },
        }),
    )
}

fn verify_write_auth<T: serde::Serialize>(
    state: &AppState,
    headers: &HeaderMap,
    payload: &T,
) -> Result<(), AppError> {
    if !state.config.internal_auth_enabled && !state.config.wallet_auth_enabled {
        return Ok(());
    }

    let internal_sig = headers
        .get("x-internal-signature")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty());
    if let Some(sig) = internal_sig {
        if !state.config.internal_auth_enabled {
            return Err(AppError::unauthorized(
                "AUTH_INTERNAL_DISABLED",
                "internal signature auth is disabled",
            ));
        }
        let secret = state
            .config
            .internal_auth_secret
            .as_deref()
            .ok_or_else(|| {
                AppError::internal("AUTH_CONFIG_ERROR", "internal auth secret missing")
            })?;
        let canonical = serde_json::to_string(payload).map_err(|e| {
            AppError::internal(
                "AUTH_SERIALIZE_ERROR",
                format!("auth payload serialization failed: {e}"),
            )
        })?;
        return verify_internal_signature(&canonical, sig, secret)
            .map_err(|e| AppError::bad_request("AUTH_INVALID_SIGNATURE", e));
    }

    if state.config.wallet_auth_enabled {
        let _claims = verify_wallet_bearer_claims(state, headers)?;
        return Ok(());
    }

    if state.config.internal_auth_enabled {
        return Err(AppError::bad_request(
            "AUTH_MISSING_SIGNATURE",
            "missing x-internal-signature",
        ));
    }

    Ok(())
}

fn verify_wallet_bearer_claims(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<WalletClaims, AppError> {
    let token = extract_bearer_token(headers)?;
    let secret = state
        .config
        .wallet_jwt_secret
        .as_deref()
        .ok_or_else(|| AppError::internal("AUTH_CONFIG_ERROR", "wallet jwt secret missing"))?;
    verify_access_token(token, secret).map_err(|e| AppError::unauthorized("AUTH_INVALID_TOKEN", e))
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, AppError> {
    let raw = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            AppError::unauthorized("AUTH_MISSING_BEARER", "missing Authorization header")
        })?;
    let stripped = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .ok_or_else(|| {
            AppError::unauthorized("AUTH_BAD_BEARER", "Authorization must be Bearer token")
        })?;
    if stripped.trim().is_empty() {
        return Err(AppError::unauthorized(
            "AUTH_BAD_BEARER",
            "empty bearer token",
        ));
    }
    Ok(stripped.trim())
}

fn error_wallet_nonce(err: AppError) -> (axum::http::StatusCode, Json<WalletNonceResponse>) {
    error!(error_code = err.code, reason = %err.message, "wallet nonce rejected");
    (
        err.status,
        Json(WalletNonceResponse {
            accepted: false,
            wallet_address: String::new(),
            nonce: String::new(),
            message: String::new(),
            expires_at: 0,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_wallet_verify(err: AppError) -> (axum::http::StatusCode, Json<WalletVerifyResponse>) {
    error!(error_code = err.code, reason = %err.message, "wallet verify rejected");
    (
        err.status,
        Json(WalletVerifyResponse {
            accepted: false,
            access_token: String::new(),
            token_type: "Bearer".to_string(),
            expires_at: 0,
            wallet_address: String::new(),
            role: String::new(),
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_submit(err: AppError) -> (axum::http::StatusCode, Json<SubmitProofJobResponse>) {
    error!(error_code = err.code, reason = %err.message, "proof job rejected");
    (
        err.status,
        Json(SubmitProofJobResponse {
            accepted: false,
            idempotent: false,
            replayed: false,
            job_id: String::new(),
            workflow_run_id: String::new(),
            policy_version: String::new(),
            proof_type: String::new(),
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_get(err: AppError) -> (axum::http::StatusCode, Json<GetProofJobResponse>) {
    error!(error_code = err.code, reason = %err.message, "proof job lookup failed");
    (
        err.status,
        Json(GetProofJobResponse {
            found: false,
            job: None,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_update(err: AppError) -> (axum::http::StatusCode, Json<UpdateProofJobStatusResponse>) {
    error!(error_code = err.code, reason = %err.message, "proof job status update rejected");
    (
        err.status,
        Json(UpdateProofJobStatusResponse {
            updated: false,
            idempotent: false,
            job: None,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_retry(err: AppError) -> (axum::http::StatusCode, Json<RetryProofJobResponse>) {
    error!(error_code = err.code, reason = %err.message, "proof job retry rejected");
    (
        err.status,
        Json(RetryProofJobResponse {
            accepted: false,
            job_id: String::new(),
            status: None,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_orchestration(
    err: AppError,
) -> (axum::http::StatusCode, Json<StartOtcOrchestrationResponse>) {
    error!(error_code = err.code, reason = %err.message, "otc orchestration rejected");
    (
        err.status,
        Json(StartOtcOrchestrationResponse {
            accepted: false,
            workflow_run_id: String::new(),
            policy_version: String::new(),
            policy_hash: String::new(),
            attestation_id: String::new(),
            attestation_hash: String::new(),
            intent_submissions: Vec::new(),
            compliance_results: Vec::new(),
            proof_job: None,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn build_compliance_results(
    requested_subjects: &[super::schema::OrchestrationSubjectInput],
    normalized_subjects: Option<&serde_json::Value>,
    decision: &str,
    error_code: Option<&str>,
) -> Vec<OtcComplianceSubjectResult> {
    let passed = decision.eq_ignore_ascii_case("PASS");
    let reason_code = if passed {
        None
    } else {
        Some(
            error_code
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("COMPLIANCE_{}", decision.to_uppercase())),
        )
    };

    if let Some(arr) = normalized_subjects.and_then(|v| v.as_array()) {
        let out = arr
            .iter()
            .filter_map(|item| item.get("subject_id").and_then(|v| v.as_str()))
            .map(|subject_id| OtcComplianceSubjectResult {
                subject_id: subject_id.to_string(),
                passed,
                decision: decision.to_string(),
                reason_code: reason_code.clone(),
            })
            .collect::<Vec<_>>();
        if !out.is_empty() {
            return out;
        }
    }

    requested_subjects
        .iter()
        .enumerate()
        .map(|(i, subject)| {
            let subject_id = subject
                .counterparty
                .as_ref()
                .map(|v| v.counterparty_id.clone())
                .or_else(|| subject.entity.as_ref().map(|v| v.entity_id.clone()))
                .unwrap_or_else(|| format!("subject-{}", i + 1));
            OtcComplianceSubjectResult {
                subject_id,
                passed,
                decision: decision.to_string(),
                reason_code: reason_code.clone(),
            }
        })
        .collect()
}
