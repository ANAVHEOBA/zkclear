use super::crud;
use super::error::AppError;
use super::schema::{
    GetProofJobResponse, GetProofJobsByRunResponse, HealthMetricsView, HealthResponse,
    OtcIntentSubmitResult, QueueStatsResponse, RetryProofJobResponse, StartOtcOrchestrationRequest,
    StartOtcOrchestrationResponse, SubmitProofJobRequest, SubmitProofJobResponse,
    UpdateProofJobStatusRequest, UpdateProofJobStatusResponse,
};
use crate::app::AppState;
use crate::service::internal_auth_service::verify_internal_signature;
use crate::service::metrics_service;
use crate::service::queue_service;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use serde_json::json;
use tracing::{error, info};
use uuid::Uuid;

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
        let payload = match response.json::<serde_json::Value>().await {
            Ok(v) => v,
            Err(e) => {
                return error_orchestration(AppError::internal(
                    "INTENT_GATEWAY_DECODE_ERROR",
                    e.to_string(),
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
    if !compliance_accepted {
        return error_orchestration(AppError::internal(
            "COMPLIANCE_REJECTED",
            compliance_json
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("compliance rejected")
                .to_string(),
        ));
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
            attestation_id,
            attestation_hash,
            intent_submissions,
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
    if !state.config.internal_auth_enabled {
        return Ok(());
    }
    let secret = state
        .config
        .internal_auth_secret
        .as_deref()
        .ok_or_else(|| AppError::internal("AUTH_CONFIG_ERROR", "internal auth secret missing"))?;
    let sig = headers
        .get("x-internal-signature")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            AppError::bad_request("AUTH_MISSING_SIGNATURE", "missing x-internal-signature")
        })?;
    let canonical = serde_json::to_string(payload).map_err(|e| {
        AppError::internal(
            "AUTH_SERIALIZE_ERROR",
            format!("auth payload serialization failed: {e}"),
        )
    })?;
    verify_internal_signature(&canonical, sig, secret)
        .map_err(|e| AppError::bad_request("AUTH_INVALID_SIGNATURE", e))
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
            attestation_id: String::new(),
            attestation_hash: String::new(),
            intent_submissions: Vec::new(),
            proof_job: None,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}
