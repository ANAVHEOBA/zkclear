use crate::config::environment::AppConfig;
use crate::infra::InfraClients;
use crate::module::policy_snapshot::crud::PolicyStore;
use crate::module::policy_snapshot::route::register_routes;
use axum::Router;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub store: Arc<PolicyStore>,
    pub infra: Option<InfraClients>,
}

impl AppState {
    pub fn new(config: AppConfig, infra: Option<InfraClients>) -> Self {
        Self {
            config,
            store: Arc::new(PolicyStore::default()),
            infra,
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    register_routes(state)
}
