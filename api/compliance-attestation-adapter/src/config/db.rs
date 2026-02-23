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
    pub fn from_app(config: &AppConfig) -> Option<Self> {
        let url = config.mongodb_url.clone()?;
        let database = config.mongodb_database.clone()?;
        Some(Self { url, database })
    }
}

impl RedisConfig {
    pub fn from_app(config: &AppConfig) -> Option<Self> {
        let url = config.redis_url.clone()?;
        Some(Self { url })
    }
}
