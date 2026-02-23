use crate::app::AppState;
use crate::module::compliance_attestation::controller;
use axum::routing::{get, post};
use axum::Router;

pub fn register_routes(state: AppState) -> Router {
    Router::new()
        .route("/v1/compliance/intake", post(controller::attest_compliance))
        .route("/v1/compliance/attest", post(controller::attest_compliance))
        .route(
            "/v1/compliance/attest/:attestation_id",
            get(controller::get_attestation),
        )
        .route("/v1/compliance/health", get(controller::health))
        .with_state(state)
}
