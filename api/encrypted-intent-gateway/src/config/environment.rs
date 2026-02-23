use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub rust_env: String,
    pub api_host: String,
    pub api_port: u16,
    pub mongodb_url: String,
    pub mongodb_database: String,
    pub redis_url: String,
    pub intent_max_age_seconds: i64,
    pub intent_max_future_skew_seconds: i64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        load_dotenv_layers();

        let rust_env = read_var("RUST_ENV")?;
        let api_host = read_var("API_HOST")?;
        let api_port = read_var("API_PORT")?
            .parse::<u16>()
            .map_err(|e| format!("invalid API_PORT: {e}"))?;
        let mongodb_url = read_var("MONGODB_URL")?;
        let mongodb_database = read_var("MONGODB_DATABASE")?;
        let redis_url = read_var("REDIS_URL")?;
        let intent_max_age_seconds = read_optional_i64("INTENT_MAX_AGE_SECONDS", 300)?;
        let intent_max_future_skew_seconds =
            read_optional_i64("INTENT_MAX_FUTURE_SKEW_SECONDS", 30)?;

        Ok(Self {
            rust_env,
            api_host,
            api_port,
            mongodb_url,
            mongodb_database,
            redis_url,
            intent_max_age_seconds,
            intent_max_future_skew_seconds,
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

fn load_dotenv_layers() {
    for path in [".env", "../.env", "../../.env"] {
        let _ = dotenvy::from_path(path);
    }
}
