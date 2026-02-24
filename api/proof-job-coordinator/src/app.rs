use crate::config::environment::AppConfig;
use crate::infra::InfraClients;
use crate::module::proof_job::crud::ProofJobStore;
use crate::module::proof_job::route::register_routes;
use axum::Router;
use axum::http::Method;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Clone)]
pub struct WalletNonceChallenge {
    pub nonce: String,
    pub message: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub store: Arc<ProofJobStore>,
    pub infra: Option<InfraClients>,
    pub wallet_nonces: Arc<RwLock<HashMap<String, WalletNonceChallenge>>>,
}

impl AppState {
    pub fn new(config: AppConfig, infra: Option<InfraClients>) -> Self {
        Self {
            config,
            store: Arc::new(ProofJobStore::default()),
            infra,
            wallet_nonces: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse().expect("valid origin"),
            "http://127.0.0.1:3000".parse().expect("valid origin"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    register_routes(state).layer(cors)
}
