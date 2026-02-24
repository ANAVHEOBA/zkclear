use super::controller;
use crate::app::AppState;
use axum::Router;
use axum::routing::{get, post};

pub fn register_routes(state: AppState) -> Router {
    Router::new()
        .route("/v1/auth/wallet/nonce", post(controller::wallet_nonce))
        .route("/v1/auth/wallet/verify", post(controller::wallet_verify))
        .route("/v1/auth/wallet/me", get(controller::wallet_me))
        .route("/v1/proof-jobs", post(controller::submit_proof_job))
        .route(
            "/v1/orchestrations/otc",
            post(controller::start_otc_orchestration),
        )
        .route("/v1/proof-jobs/health", get(controller::health))
        .route(
            "/v1/proof-jobs/queue-stats",
            get(controller::get_queue_stats),
        )
        .route("/v1/proof-jobs/:job_id", get(controller::get_proof_job))
        .route(
            "/v1/proof-jobs/run/:workflow_run_id",
            get(controller::get_proof_jobs_by_run),
        )
        .route(
            "/v1/proof-jobs/:job_id/retry",
            post(controller::retry_proof_job),
        )
        .route(
            "/v1/proof-jobs/:job_id/status",
            post(controller::update_proof_job_status),
        )
        .with_state(state)
}
