use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub rust_env: String,
    pub api_host: String,
    pub api_port: u16,
    pub mongodb_url: Option<String>,
    pub mongodb_database: Option<String>,
    pub redis_url: Option<String>,
    pub intake_max_age_seconds: i64,
    pub intake_max_future_skew_seconds: i64,
    pub sanctions_data_path: String,
    pub policy_snapshot_path: String,
    pub policy_version: String,
    pub attestation_ttl_seconds: i64,
    pub replay_ttl_seconds: i64,
    pub require_internal_signature: bool,
    pub internal_signing_secret: Option<String>,
    pub encryption_key_hex: Option<String>,
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
            mongodb_url: read_optional_env("MONGODB_URL"),
            mongodb_database: read_optional_env("MONGODB_DATABASE"),
            redis_url: read_optional_env("REDIS_URL"),
            intake_max_age_seconds: read_optional_i64("INTAKE_MAX_AGE_SECONDS", 300)?,
            intake_max_future_skew_seconds: read_optional_i64("INTAKE_MAX_FUTURE_SKEW_SECONDS", 30)?,
            sanctions_data_path: read_optional_string("SANCTIONS_DATA_PATH", "data/sanctions.json"),
            policy_snapshot_path: read_optional_string(
                "POLICY_SNAPSHOT_PATH",
                "config/policy_snapshot.json",
            ),
            policy_version: read_optional_string("POLICY_VERSION", "policy-v1"),
            attestation_ttl_seconds: read_optional_i64("ATTESTATION_TTL_SECONDS", 3600)?,
            replay_ttl_seconds: read_optional_i64("REPLAY_TTL_SECONDS", 86400)?,
            require_internal_signature: read_optional_bool("REQUIRE_INTERNAL_SIGNATURE", false),
            internal_signing_secret: read_optional_env("INTERNAL_SIGNING_SECRET"),
            encryption_key_hex: read_optional_env("ENCRYPTION_KEY_HEX"),
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

fn read_optional_string(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn read_optional_env(key: &str) -> Option<String> {
    env::var(key).ok()
}

fn read_optional_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => default,
    }
}

fn load_dotenv_layers() {
    for path in [".env", "../.env", "../../.env"] {
        let _ = dotenvy::from_path(path);
    }
}
