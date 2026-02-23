use crate::config::environment::AppConfig;
use mongodb::Client as MongoClient;
use mongodb::Database;
use redis::Client as RedisClient;

pub const SNAPSHOTS_COLLECTION: &str = "policy_snapshots";
pub const ACTIVATION_HISTORY_COLLECTION: &str = "policy_activation_history";
pub const RUN_EVIDENCE_COLLECTION: &str = "policy_run_evidence";
pub const AUDIT_LOG_COLLECTION: &str = "policy_audit_log";

#[derive(Debug, Clone)]
pub struct InfraClients {
    pub mongo_db: Database,
    pub redis: RedisClient,
}

pub async fn init_infra(config: &AppConfig) -> Result<Option<InfraClients>, String> {
    let Some(mongo_url) = &config.mongodb_url else {
        return Ok(None);
    };
    let Some(mongo_db_name) = &config.mongodb_database else {
        return Ok(None);
    };
    let Some(redis_url) = &config.redis_url else {
        return Ok(None);
    };

    let mongo_client = MongoClient::with_uri_str(mongo_url)
        .await
        .map_err(|e| format!("mongodb client init failed: {e}"))?;
    let mongo_db = mongo_client.database(mongo_db_name);

    let redis = RedisClient::open(redis_url.clone()).map_err(|e| format!("redis client init failed: {e}"))?;
    Ok(Some(InfraClients { mongo_db, redis }))
}
