use axum::Router;
use axum::routing::post;

use crate::app::AppState;
use crate::module::encrypted_intent::controller;

pub fn register_routes(state: AppState) -> Router {
    Router::new()
        .route("/v1/intents/submit", post(controller::submit_intent))
        .with_state(state)
}
