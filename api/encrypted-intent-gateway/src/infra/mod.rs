use crate::config::db::{MongoConfig, RedisConfig};
use mongodb::IndexModel;
use mongodb::bson::doc;
use mongodb::options::IndexOptions;
use mongodb::Client as MongoClient;
use mongodb::Database;
use redis::Client as RedisClient;
use redis::cmd;

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
    ensure_indexes(&mongo_db).await?;

    let redis_client =
        RedisClient::open(redis.url.clone()).map_err(|e| format!("redis client init failed: {e}"))?;
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connection failed: {e}"))?;
    let pong: String = cmd("PING")
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("redis ping failed: {e}"))?;
    if pong != "PONG" {
        return Err("redis ping returned unexpected response".to_string());
    }

    Ok(InfraClients {
        mongo_db,
        redis: redis_client,
    })
}

async fn ensure_indexes(db: &Database) -> Result<(), String> {
    let collection = db.collection::<mongodb::bson::Document>("encrypted_intents");
    let unique = IndexOptions::builder().unique(true).build();

    let indexes = vec![
        IndexModel::builder()
            .keys(doc! { "intent_id": 1 })
            .options(unique.clone())
            .build(),
        IndexModel::builder()
            .keys(doc! { "nonce": 1 })
            .options(unique.clone())
            .build(),
        IndexModel::builder()
            .keys(doc! { "commitment_hash": 1 })
            .options(unique)
            .build(),
    ];

    collection
        .create_indexes(indexes)
        .await
        .map_err(|e| format!("mongodb index creation failed: {e}"))?;
    Ok(())
}
