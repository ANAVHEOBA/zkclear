use crate::app::AppState;
use crate::module::policy_snapshot::controller;
use axum::routing::{get, post};
use axum::Router;

pub fn register_routes(state: AppState) -> Router {
    Router::new()
        .route("/v1/policy/snapshots", post(controller::create_snapshot))
        .route("/v1/policy/activate", post(controller::activate_policy))
        .route(
            "/v1/policy/snapshots/:policy_version",
            get(controller::get_snapshot_by_version),
        )
        .route(
            "/v1/policy/snapshots/hash/:policy_hash",
            get(controller::get_snapshot_by_hash),
        )
        .route("/v1/policy/active", get(controller::get_active_policy))
        .route(
            "/v1/policy/active/at/:timestamp",
            get(controller::get_active_policy_at_timestamp),
        )
        .route(
            "/v1/policy/effective/:run_id",
            get(controller::get_effective_policy_for_run),
        )
        .with_state(state)
}
