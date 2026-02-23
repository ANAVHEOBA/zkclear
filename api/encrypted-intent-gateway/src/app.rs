use crate::config::environment::AppConfig;
use crate::infra::InfraClients;
use crate::module::encrypted_intent::route::register_routes;
use axum::Router;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub infra: InfraClients,
}

impl AppState {
    pub fn new(config: AppConfig, infra: InfraClients) -> Self {
        Self { config, infra }
    }
}

pub fn build_router(state: AppState) -> Router {
    register_routes(state)
}
