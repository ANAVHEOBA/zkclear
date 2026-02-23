use super::crud;
use super::error::AppError;
use super::schema::{
    GetProofJobResponse, GetProofJobsByRunResponse, HealthMetricsView, HealthResponse,
    QueueStatsResponse, RetryProofJobResponse, SubmitProofJobRequest, SubmitProofJobResponse,
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
