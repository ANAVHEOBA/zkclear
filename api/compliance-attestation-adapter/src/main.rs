use compliance_attestation_adapter::app::{AppState, build_router};
use compliance_attestation_adapter::config::db::{MongoConfig, RedisConfig};
use compliance_attestation_adapter::config::environment::AppConfig;
use compliance_attestation_adapter::infra::init_infra;
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
        "compliance-attestation-adapter started"
    );

    let infra = match (
        MongoConfig::from_app(&config),
        RedisConfig::from_app(&config),
    ) {
        (Some(mongo), Some(redis)) => match init_infra(&mongo, &redis).await {
            Ok(i) => Some(i),
            Err(e) => {
                warn!(error = %e, "infra init failed; running without persistence");
                None
            }
        },
        _ => {
            warn!("mongo/redis env not fully configured; running without persistence");
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
