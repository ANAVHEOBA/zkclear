pub mod mongo;
pub mod provider_client;
pub mod redis;

use crate::config::db::{MongoConfig, RedisConfig};
use ::redis::Client as RedisClient;
use mongodb::Client as MongoClient;
use mongodb::Database;

#[derive(Debug, Clone)]
pub struct InfraClients {
    pub mongo_db: Database,
    pub redis: RedisClient,
}

pub async fn init_infra(mongo: &MongoConfig, redis: &RedisConfig) -> Result<InfraClients, String> {
    let mongo_client = MongoClient::with_uri_str(&mongo.url)
        .await
        .map_err(|e| format!("mongodb client init failed: {e}"))?;
    let mongo_db = mongo_client.database(&mongo.database);

    let redis_client = RedisClient::open(redis.url.clone())
        .map_err(|e| format!("redis client init failed: {e}"))?;

    Ok(InfraClients {
        mongo_db,
        redis: redis_client,
    })
}
