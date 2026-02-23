use crate::config::environment::AppConfig;
use crate::infra::InfraClients;
use crate::module::proof_job::crud::ProofJobStore;
use crate::module::proof_job::route::register_routes;
use axum::Router;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub store: Arc<ProofJobStore>,
    pub infra: Option<InfraClients>,
}

impl AppState {
    pub fn new(config: AppConfig, infra: Option<InfraClients>) -> Self {
        Self {
            config,
            store: Arc::new(ProofJobStore::default()),
            infra,
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    register_routes(state)
}
