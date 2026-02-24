use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub rust_env: String,
    pub api_host: String,
    pub api_port: u16,
    pub mongodb_url: Option<String>,
    pub mongodb_database: Option<String>,
    pub redis_url: Option<String>,
    pub idempotency_ttl_seconds: i64,
    pub worker_enabled: bool,
    pub worker_poll_seconds: i64,
    pub worker_lease_seconds: i64,
    pub worker_max_retries: i64,
    pub worker_backoff_base_seconds: i64,
    pub zk_root_dir: String,
    pub prove_timeout_seconds: i64,
    pub prove_budget_settlement_seconds: i64,
    pub prove_budget_compliance_seconds: i64,
    pub prove_budget_rebate_seconds: i64,
    pub signal_domain_separator: String,
    pub eth_sepolia_rpc_url: Option<String>,
    pub private_key: Option<String>,
    pub eth_sepolia_chain_id: u64,
    pub publish_settlement_registry: Option<String>,
    pub publish_publisher_address: Option<String>,
    pub internal_auth_enabled: bool,
    pub internal_auth_secret: Option<String>,
    pub wallet_auth_enabled: bool,
    pub wallet_auth_nonce_ttl_seconds: i64,
    pub wallet_jwt_secret: Option<String>,
    pub wallet_jwt_ttl_seconds: i64,
    pub wallet_role_map: String,
    pub wallet_default_role: String,
    pub intent_gateway_base_url: String,
    pub compliance_adapter_base_url: String,
    pub policy_snapshot_base_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        load_dotenv_layers();
        Ok(Self {
            rust_env: read_var("RUST_ENV")?,
            api_host: read_var("API_HOST")?,
            api_port: read_var("API_PORT")?
                .parse::<u16>()
                .map_err(|e| format!("invalid API_PORT: {e}"))?,
            mongodb_url: env::var("MONGODB_URL").ok(),
            mongodb_database: env::var("MONGODB_DATABASE").ok(),
            redis_url: env::var("REDIS_URL").ok(),
            idempotency_ttl_seconds: read_optional_i64("IDEMPOTENCY_TTL_SECONDS", 3600)?,
            worker_enabled: read_optional_bool("WORKER_ENABLED", true),
            worker_poll_seconds: read_optional_i64("WORKER_POLL_SECONDS", 2)?,
            worker_lease_seconds: read_optional_i64("WORKER_LEASE_SECONDS", 30)?,
            worker_max_retries: read_optional_i64("WORKER_MAX_RETRIES", 3)?,
            worker_backoff_base_seconds: read_optional_i64("WORKER_BACKOFF_BASE_SECONDS", 2)?,
            zk_root_dir: read_optional_string("ZK_ROOT_DIR", "../../zk"),
            prove_timeout_seconds: read_optional_i64("PROVE_TIMEOUT_SECONDS", 60)?,
            prove_budget_settlement_seconds: read_optional_i64(
                "PROVE_BUDGET_SETTLEMENT_SECONDS",
                15,
            )?,
            prove_budget_compliance_seconds: read_optional_i64(
                "PROVE_BUDGET_COMPLIANCE_SECONDS",
                10,
            )?,
            prove_budget_rebate_seconds: read_optional_i64("PROVE_BUDGET_REBATE_SECONDS", 10)?,
            signal_domain_separator: read_optional_string("SIGNAL_DOMAIN_SEPARATOR", "zkclear:v1"),
            eth_sepolia_rpc_url: env::var("ETH_SEPOLIA_RPC_URL").ok(),
            private_key: env::var("PRIVATE_KEY").ok(),
            eth_sepolia_chain_id: read_optional_u64("ETH_SEPOLIA_CHAIN_ID", 11155111)?,
            publish_settlement_registry: env::var("PUBLISH_SETTLEMENT_REGISTRY").ok(),
            publish_publisher_address: env::var("PUBLISH_PUBLISHER_ADDRESS").ok(),
            internal_auth_enabled: read_optional_bool("INTERNAL_AUTH_ENABLED", false),
            internal_auth_secret: env::var("INTERNAL_AUTH_SECRET").ok(),
            wallet_auth_enabled: read_optional_bool("WALLET_AUTH_ENABLED", true),
            wallet_auth_nonce_ttl_seconds: read_optional_i64("WALLET_AUTH_NONCE_TTL_SECONDS", 300)?,
            wallet_jwt_secret: env::var("WALLET_JWT_SECRET").ok(),
            wallet_jwt_ttl_seconds: read_optional_i64("WALLET_JWT_TTL_SECONDS", 3600)?,
            wallet_role_map: read_optional_string("WALLET_ROLE_MAP", ""),
            wallet_default_role: read_optional_string("WALLET_DEFAULT_ROLE", "dealer"),
            intent_gateway_base_url: read_optional_string(
                "INTENT_GATEWAY_BASE_URL",
                "http://127.0.0.1:8080",
            ),
            compliance_adapter_base_url: read_optional_string(
                "COMPLIANCE_ADAPTER_BASE_URL",
                "http://127.0.0.1:8082",
            ),
            policy_snapshot_base_url: read_optional_string(
                "POLICY_SNAPSHOT_BASE_URL",
                "http://127.0.0.1:8083",
            ),
        })
    }
}

fn read_var(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("missing required env var: {key}"))
}

fn read_optional_i64(key: &str, default: i64) -> Result<i64, String> {
    match env::var(key) {
        Ok(v) => v.parse::<i64>().map_err(|e| format!("invalid {key}: {e}")),
        Err(_) => Ok(default),
    }
}

fn read_optional_u64(key: &str, default: u64) -> Result<u64, String> {
    match env::var(key) {
        Ok(v) => v.parse::<u64>().map_err(|e| format!("invalid {key}: {e}")),
        Err(_) => Ok(default),
    }
}

fn read_optional_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => default,
    }
}

fn read_optional_string(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn load_dotenv_layers() {
    for path in [".env", "../.env", "../../.env"] {
        let _ = dotenvy::from_path_override(path);
    }
}
