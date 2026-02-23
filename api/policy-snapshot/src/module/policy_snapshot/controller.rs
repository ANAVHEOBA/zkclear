use super::crud;
use super::error::AppError;
use super::schema::{
    ActivatePolicyRequest, ActivatePolicyResponse, ActivePolicyResponse, CreateSnapshotRequest,
    CreateSnapshotResponse, EffectivePolicyQuery, EffectivePolicyResponse, SnapshotLookupResponse,
};
use crate::app::AppState;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::{Json, response::IntoResponse};
use tracing::error;

pub async fn create_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateSnapshotRequest>,
) -> impl IntoResponse {
    match crud::create_snapshot(&state, headers, req).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(err) => error_snapshot(err),
    }
}

pub async fn activate_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ActivatePolicyRequest>,
) -> impl IntoResponse {
    match crud::activate_policy(&state, headers, req).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(err) => error_activate(err),
    }
}

pub async fn get_snapshot_by_version(
    State(state): State<AppState>,
    Path(policy_version): Path<String>,
) -> impl IntoResponse {
    match crud::get_snapshot_by_version(&state, &policy_version).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(err) => error_lookup(err),
    }
}

pub async fn get_snapshot_by_hash(
    State(state): State<AppState>,
    Path(policy_hash): Path<String>,
) -> impl IntoResponse {
    match crud::get_snapshot_by_hash(&state, &policy_hash).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(err) => error_lookup(err),
    }
}

pub async fn get_active_policy(State(state): State<AppState>) -> impl IntoResponse {
    match crud::get_active_policy(&state).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(err) => error_active(err),
    }
}

pub async fn get_active_policy_at_timestamp(
    State(state): State<AppState>,
    Path(timestamp): Path<i64>,
) -> impl IntoResponse {
    match crud::get_active_policy_at_timestamp(&state, timestamp).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(err) => error_active(err),
    }
}

pub async fn get_effective_policy_for_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Query(query): Query<EffectivePolicyQuery>,
) -> impl IntoResponse {
    match crud::get_effective_policy_for_run(
        &state.config,
        &state.store,
        state.infra.as_ref(),
        &run_id,
        query.timestamp,
        query.version_hint,
    )
    .await
    {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)),
        Err(err) => error_effective(err, run_id),
    }
}

fn error_snapshot(err: AppError) -> (axum::http::StatusCode, Json<CreateSnapshotResponse>) {
    error!(error_code = err.code, reason = %err.message, "create snapshot rejected");
    (
        err.status,
        Json(CreateSnapshotResponse {
            accepted: false,
            idempotent: false,
            policy_version: String::new(),
            policy_hash: String::new(),
            canonical_rules_json: String::new(),
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_activate(err: AppError) -> (axum::http::StatusCode, Json<ActivatePolicyResponse>) {
    error!(error_code = err.code, reason = %err.message, "activate policy rejected");
    (
        err.status,
        Json(ActivatePolicyResponse {
            accepted: false,
            active_mapping: super::model::ActivePolicyRecord {
                onchain_policy_version: String::new(),
                policy_version: String::new(),
                policy_hash: String::new(),
                activated_at: 0,
                deactivated_at: None,
            },
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_lookup(err: AppError) -> (axum::http::StatusCode, Json<SnapshotLookupResponse>) {
    error!(error_code = err.code, reason = %err.message, "lookup policy rejected");
    (
        err.status,
        Json(SnapshotLookupResponse {
            found: false,
            snapshot: None,
            active_mapping: None,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_active(err: AppError) -> (axum::http::StatusCode, Json<ActivePolicyResponse>) {
    error!(error_code = err.code, reason = %err.message, "active policy lookup rejected");
    (
        err.status,
        Json(ActivePolicyResponse {
            found: false,
            active_mapping: None,
            snapshot: None,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}

fn error_effective(
    err: AppError,
    run_id: String,
) -> (axum::http::StatusCode, Json<EffectivePolicyResponse>) {
    error!(error_code = err.code, reason = %err.message, "effective policy lookup rejected");
    (
        err.status,
        Json(EffectivePolicyResponse {
            found: false,
            run_id,
            snapshot: None,
            activation: None,
            evidence: None,
            error_code: Some(err.code.to_string()),
            reason: err.message,
        }),
    )
}
