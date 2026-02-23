use crate::config::environment::AppConfig;

#[derive(Debug, Clone)]
pub struct MongoConfig {
    pub url: String,
    pub database: String,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
}

impl MongoConfig {
    pub fn from_app(app: &AppConfig) -> Self {
        Self {
            url: app.mongodb_url.clone().unwrap_or_default(),
            database: app
                .mongodb_database
                .clone()
                .unwrap_or_else(|| "zkclear".to_string()),
        }
    }
}

impl RedisConfig {
    pub fn from_app(app: &AppConfig) -> Self {
        Self {
            url: app.redis_url.clone().unwrap_or_default(),
        }
    }
}
