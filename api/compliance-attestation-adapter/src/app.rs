use crate::config::environment::AppConfig;
use crate::infra::InfraClients;
use crate::module::compliance_attestation::route::register_routes;
use axum::Router;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub infra: Option<InfraClients>,
}

impl AppState {
    pub fn new(config: AppConfig, infra: Option<InfraClients>) -> Self {
        Self { config, infra }
    }
}

pub fn build_router(state: AppState) -> Router {
    register_routes(state)
}
