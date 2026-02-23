use crate::config::environment::AppConfig;
use redis::Client as RedisClient;

#[derive(Debug, Clone)]
pub struct InfraClients {
    pub redis: RedisClient,
}

pub const PROOF_JOBS_COLLECTION: &str = "proof_jobs";
pub const PROOF_JOB_ATTEMPTS_COLLECTION: &str = "proof_job_attempts";
pub const PROOF_OUTPUTS_COLLECTION: &str = "proof_outputs";
pub const PUBLISH_RECEIPTS_COLLECTION: &str = "publish_receipts";

pub async fn init_infra(config: &AppConfig) -> Result<Option<InfraClients>, String> {
    let Some(redis_url) = &config.redis_url else {
        return Ok(None);
    };

    let redis =
        RedisClient::open(redis_url.clone()).map_err(|e| format!("redis init failed: {e}"))?;
    Ok(Some(InfraClients { redis }))
}
