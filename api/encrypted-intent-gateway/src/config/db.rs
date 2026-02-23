use super::environment::AppConfig;

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
    pub fn from_app(config: &AppConfig) -> Self {
        Self {
            url: config.mongodb_url.clone(),
            database: config.mongodb_database.clone(),
        }
    }
}

impl RedisConfig {
    pub fn from_app(config: &AppConfig) -> Self {
        Self {
            url: config.redis_url.clone(),
        }
    }
}

