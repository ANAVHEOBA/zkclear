use policy_snapshot::app::{AppState, build_router};
use policy_snapshot::config::environment::AppConfig;
use policy_snapshot::infra::init_infra;
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
        "policy-snapshot started"
    );

    let infra = match init_infra(&config).await {
        Ok(i) => i,
        Err(e) => {
            warn!(error = %e, "infra init failed; running without external storage");
            None
        }
    };

    let state = AppState::new(config, infra);
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
