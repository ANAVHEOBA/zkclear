use encrypted_intent_gateway::app::{AppState, build_router};
use encrypted_intent_gateway::config::db::{MongoConfig, RedisConfig};
use encrypted_intent_gateway::config::environment::AppConfig;
use encrypted_intent_gateway::infra::init_infra;
use tokio::net::TcpListener;
use tracing::{error, info};

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

    let mongo = MongoConfig::from_app(&config);
    let redis = RedisConfig::from_app(&config);
    let infra = match init_infra(&mongo, &redis).await {
        Ok(infra) => infra,
        Err(e) => {
            error!(error = %e, "infra init error");
            std::process::exit(1);
        }
    };
    let state = AppState::new(config.clone(), infra);

    info!(
        "encrypted-intent-gateway config loaded: env={} host={} port={} mongo_db={}",
        state.config.rust_env, state.config.api_host, state.config.api_port, mongo.database
    );
    info!("mongo_url={} redis_url={}", mongo.url, redis.url);

    let bind_addr = format!("{}:{}", state.config.api_host, state.config.api_port);
    let listener = match TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(error = %e, bind_addr = %bind_addr, "server bind error");
            std::process::exit(1);
        }
    };

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
