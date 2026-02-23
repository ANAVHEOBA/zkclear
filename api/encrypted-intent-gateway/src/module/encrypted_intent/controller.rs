use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Json, response::IntoResponse};
use tracing::{error, info};

use crate::app::AppState;
use crate::service::metrics_service;

use super::crud;
use super::error::AppError;
use super::schema::{SubmitIntentRequest, SubmitIntentResponse};

pub async fn submit_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SubmitIntentRequest>,
) -> impl IntoResponse {
    let started = metrics_service::start_timer();
    let forced_workflow_run_id = headers
        .get("x-workflow-run-id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);
    match crud::submit_intent(&state, req, forced_workflow_run_id).await {
        Ok(resp) => {
            metrics_service::record_intent_submit_success();
            let (ok_count, err_count) = metrics_service::snapshot();
            info!(
                workflow_run_id = %resp.workflow_run_id,
                elapsed_ms = metrics_service::elapsed_ms(started),
                ok_count,
                err_count,
                "intent submit accepted"
            );
            (axum::http::StatusCode::OK, Json(resp))
        }
        Err(AppError {
            status,
            code,
            message,
        }) => {
            metrics_service::record_intent_submit_failure();
            let (ok_count, err_count) = metrics_service::snapshot();
            error!(
                error_code = code,
                reason = %message,
                elapsed_ms = metrics_service::elapsed_ms(started),
                ok_count,
                err_count,
                "intent submit rejected"
            );
            (
                status,
                Json(SubmitIntentResponse {
                    workflow_run_id: String::new(),
                    intent_ids: vec![],
                    commitment_hashes: vec![],
                    accepted: false,
                    error_code: Some(code.to_string()),
                    reason: message,
                }),
            )
        }
    }
}
