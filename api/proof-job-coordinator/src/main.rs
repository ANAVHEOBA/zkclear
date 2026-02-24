use proof_job_coordinator::app::{AppState, build_router};
use proof_job_coordinator::config::environment::AppConfig;
use proof_job_coordinator::infra::init_infra;
use proof_job_coordinator::service::queue_service;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() {
    init_logging();

    let config = match AppConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "config error");
            std::process::exit(1);
        }
    };

    let bind_addr = format!("{}:{}", config.api_host, config.api_port);
    let listener = match TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(error = %e, bind_addr = %bind_addr, "server bind error");
            std::process::exit(1);
        }
    };

    info!(
        env = %config.rust_env,
        host = %config.api_host,
        port = config.api_port,
        intent_gateway_base_url = %config.intent_gateway_base_url,
        compliance_adapter_base_url = %config.compliance_adapter_base_url,
        policy_snapshot_base_url = %config.policy_snapshot_base_url,
        "proof-job-coordinator started"
    );

    let infra = match init_infra(&config).await {
        Ok(i) => i,
        Err(e) => {
            warn!(error = %e, "infra init failed; queue worker disabled");
            None
        }
    };
    let state = AppState::new(config, infra);
    if state.config.worker_enabled && state.infra.is_some() {
        let worker_state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = queue_service::run_worker(worker_state).await {
                error!(error = %e, "queue worker exited");
            }
        });
    }
    let app = build_router(state);
    if let Err(e) = axum::serve(listener, app).await {
        error!(error = %e, "server runtime error");
        std::process::exit(1);
    }
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
