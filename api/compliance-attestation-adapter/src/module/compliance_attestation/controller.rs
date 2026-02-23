use super::crud;
use super::error::AppError;
use super::schema::{
    ComplianceDecision, HealthResponse, IntakeComplianceRequest, IntakeComplianceResponse,
};
use crate::app::AppState;
use axum::extract::{Path, State};
use axum::{Json, response::IntoResponse};
use tracing::{error, info};

pub async fn attest_compliance(
    State(state): State<AppState>,
    Json(req): Json<IntakeComplianceRequest>,
) -> impl IntoResponse {
    match crud::intake_and_normalize(&state, req).await {
        Ok(resp) => {
            info!(
                workflow_run_id = %resp.workflow_run_id,
                request_id = %resp.request_id,
                normalized_subject_count = resp.normalized_subject_count,
                decision = %resp.decision.as_str(),
                "compliance attest accepted"
            );
            (axum::http::StatusCode::OK, Json(resp))
        }
        Err(AppError {
            status,
            code,
            message,
        }) => {
            error!(error_code = code, reason = %message, "compliance attest rejected");
            (status, Json(error_response(code, message)))
        }
    }
}

pub async fn get_attestation(
    State(state): State<AppState>,
    Path(attestation_id): Path<String>,
) -> impl IntoResponse {
    match crud::get_attestation_by_id(&state, &attestation_id).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(AppError { status, code, message }) => {
            error!(error_code = code, reason = %message, "get attestation rejected");
            (status, Json(error_response(code, message)))
        }
    }
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let persistence_enabled = state.infra.is_some();
    (
        axum::http::StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
            persistence_enabled,
        }),
    )
}

fn error_response(code: &str, message: String) -> IntakeComplianceResponse {
    IntakeComplianceResponse {
        attestation_id: String::new(),
        workflow_run_id: String::new(),
        request_id: String::new(),
        accepted: false,
        normalized_subject_count: 0,
        normalized_subjects: vec![],
        policy_version: String::new(),
        policy_hash: String::new(),
        decision: ComplianceDecision::Fail,
        risk_score: 0,
        sanctions_hit_count: 0,
        attestation_hash: String::new(),
        issued_at: 0,
        expires_at: 0,
        error_code: Some(code.to_string()),
        reason: message,
    }
}
