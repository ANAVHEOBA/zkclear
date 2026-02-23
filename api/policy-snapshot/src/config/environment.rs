use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub rust_env: String,
    pub api_host: String,
    pub api_port: u16,
    pub audit_signing_secret: Option<String>,
    pub mongodb_url: Option<String>,
    pub mongodb_database: Option<String>,
    pub redis_url: Option<String>,
    pub require_internal_auth: bool,
    pub internal_auth_secret: Option<String>,
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
            audit_signing_secret: env::var("AUDIT_SIGNING_SECRET").ok(),
            mongodb_url: env::var("MONGODB_URL").ok(),
            mongodb_database: env::var("MONGODB_DATABASE").ok(),
            redis_url: env::var("REDIS_URL").ok(),
            require_internal_auth: read_optional_bool("REQUIRE_INTERNAL_AUTH", false),
            internal_auth_secret: env::var("INTERNAL_AUTH_SECRET").ok(),
        })
    }
}

fn read_var(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("missing required env var: {key}"))
}

fn load_dotenv_layers() {
    for path in [".env", "../.env", "../../.env"] {
        let _ = dotenvy::from_path(path);
    }
}

fn read_optional_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => default,
    }
}
