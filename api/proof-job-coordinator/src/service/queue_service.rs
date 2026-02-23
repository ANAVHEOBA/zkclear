use crate::app::AppState;
use crate::module::proof_job::crud;
use crate::module::proof_job::schema::{JobStatus, UpdateProofJobStatusRequest};
use crate::service::metrics_service;
use crate::service::prover_service;
use crate::service::publish_service;
use crate::service::signal_binding_service;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{error, info, warn};

const QUEUE_KEY: &str = "proofjobs:queue";
const PROCESSING_KEY: &str = "proofjobs:processing";
const RETRY_ZSET_KEY: &str = "proofjobs:retry";
const DLQ_KEY: &str = "proofjobs:dead";
const ATTEMPTS_HASH_KEY: &str = "proofjobs:attempts";
const LOCK_PREFIX: &str = "proofjobs:lock:";

pub async fn enqueue_proof_job(state: &AppState, job_id: &str) -> Result<(), String> {
    let Some(infra) = &state.infra else {
        return Ok(());
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect failed: {e}"))?;
    let _: usize = conn
        .lpush(QUEUE_KEY, job_id)
        .await
        .map_err(|e| format!("queue push failed: {e}"))?;
    Ok(())
}

pub async fn run_worker(state: AppState) -> Result<(), String> {
    info!("proof queue worker started");
    loop {
        if let Err(e) = promote_due_retries(&state).await {
            warn!(error = %e, "retry promotion failed");
        }

        let Some(job_id) = pull_next_job(&state).await? else {
            sleep(Duration::from_secs(
                state.config.worker_poll_seconds.max(1) as u64
            ))
            .await;
            continue;
        };

        if let Err(e) = process_job_with_lease(&state, &job_id).await {
            warn!(job_id = %job_id, error = %e, "job processing failed");
            if let Err(retry_err) = handle_failure(&state, &job_id, &e).await {
                error!(job_id = %job_id, error = %retry_err, "retry handling failed");
            }
        }
    }
}

pub async fn queue_stats(state: &AppState) -> Result<(u64, u64, u64, u64), String> {
    let Some(infra) = &state.infra else {
        return Err("redis is not configured".to_string());
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect failed: {e}"))?;

    let queued: u64 = conn
        .llen(QUEUE_KEY)
        .await
        .map_err(|e| format!("queue len failed: {e}"))?;
    let processing: u64 = conn
        .llen(PROCESSING_KEY)
        .await
        .map_err(|e| format!("processing len failed: {e}"))?;
    let retry_scheduled: u64 = conn
        .zcard(RETRY_ZSET_KEY)
        .await
        .map_err(|e| format!("retry zcard failed: {e}"))?;
    let dead_letter: u64 = conn
        .llen(DLQ_KEY)
        .await
        .map_err(|e| format!("dead-letter len failed: {e}"))?;
    Ok((queued, processing, retry_scheduled, dead_letter))
}

async fn pull_next_job(state: &AppState) -> Result<Option<String>, String> {
    let Some(infra) = &state.infra else {
        return Ok(None);
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect failed: {e}"))?;

    let res: Option<String> = redis::cmd("BRPOPLPUSH")
        .arg(QUEUE_KEY)
        .arg(PROCESSING_KEY)
        .arg(1)
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("queue pull failed: {e}"))?;
    Ok(res)
}

async fn process_job_with_lease(state: &AppState, job_id: &str) -> Result<(), String> {
    let Some(infra) = &state.infra else {
        return Ok(());
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect failed: {e}"))?;
    let lock_key = format!("{LOCK_PREFIX}{job_id}");
    let lock_token = format!("{}-{}", job_id, now_unix());

    let acquired: Option<String> = redis::cmd("SET")
        .arg(&lock_key)
        .arg(&lock_token)
        .arg("NX")
        .arg("EX")
        .arg(state.config.worker_lease_seconds.max(5))
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("lease acquisition failed: {e}"))?;

    if acquired.is_none() {
        release_processing_entry(&mut conn, job_id).await?;
        return Ok(());
    }

    let result = process_job(state, job_id).await;
    if let Err(err) = &result {
        warn!(job_id = %job_id, error = %err, "processing logic returned error");
    }

    let _: usize = redis::cmd("DEL")
        .arg(&lock_key)
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("lock release failed: {e}"))?;

    if result.is_ok() {
        release_processing_entry(&mut conn, job_id).await?;
        let _: usize = conn
            .hdel(ATTEMPTS_HASH_KEY, job_id)
            .await
            .map_err(|e| format!("attempt reset failed: {e}"))?;
    }

    result
}

async fn process_job(state: &AppState, job_id: &str) -> Result<(), String> {
    let proving = UpdateProofJobStatusRequest {
        next_status: JobStatus::Proving,
        error_code: None,
        error_message: None,
    };
    crud::update_proof_job_status(state, job_id, proving)
        .await
        .map_err(|e| e.message.clone())?;

    let job = crud::get_proof_job_record(state, job_id)
        .await
        .map_err(|e| e.message)?;
    if job.created_at > 0 {
        let now = now_unix();
        if now > job.created_at {
            metrics_service::record_queue_latency_ms(((now - job.created_at) * 1000) as u64);
        }
    }
    let artifacts = prover_service::run_prover_for_job(state, &job)
        .await
        .map_err(|e| format!("prover failed: {e}"))?;
    metrics_service::record_prove_duration_ms((artifacts.prove_time_seconds.max(0) as u64) * 1000);
    crud::set_prover_artifacts(state, job_id, artifacts)
        .await
        .map_err(|e| e.message.clone())?;
    let job_with_artifacts = crud::get_proof_job_record(state, job_id)
        .await
        .map_err(|e| e.message)?;
    let stored_artifacts = job_with_artifacts
        .prover_artifacts
        .as_ref()
        .ok_or_else(|| "prover artifacts missing after write".to_string())?;
    signal_binding_service::validate_public_signal_binding(
        state,
        &job_with_artifacts,
        stored_artifacts,
    )?;

    let proved = UpdateProofJobStatusRequest {
        next_status: JobStatus::Proved,
        error_code: None,
        error_message: None,
    };
    crud::update_proof_job_status(state, job_id, proved)
        .await
        .map_err(|e| e.message.clone())?;

    let publishing = UpdateProofJobStatusRequest {
        next_status: JobStatus::Publishing,
        error_code: None,
        error_message: None,
    };
    crud::update_proof_job_status(state, job_id, publishing)
        .await
        .map_err(|e| e.message.clone())?;

    let publish_job = crud::get_proof_job_record(state, job_id)
        .await
        .map_err(|e| e.message)?;
    let publish_result = publish_service::publish_receipt_on_sepolia(state, &publish_job).await?;
    crud::set_onchain_publish_result(state, job_id, publish_result)
        .await
        .map_err(|e| e.message.clone())?;

    let published = UpdateProofJobStatusRequest {
        next_status: JobStatus::Published,
        error_code: None,
        error_message: None,
    };
    crud::update_proof_job_status(state, job_id, published)
        .await
        .map_err(|e| e.message.clone())?;
    Ok(())
}

async fn handle_failure(state: &AppState, job_id: &str, reason: &str) -> Result<(), String> {
    let Some(infra) = &state.infra else {
        return Ok(());
    };
    let mut conn: MultiplexedConnection = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect failed: {e}"))?;
    let attempts: i64 = conn
        .hincr(ATTEMPTS_HASH_KEY, job_id, 1)
        .await
        .map_err(|e| format!("attempt increment failed: {e}"))?;

    if let Some((code, message)) = parse_non_retryable(reason) {
        metrics_service::set_last_error_ts(now_unix());
        let _ = crud::update_proof_job_status(
            state,
            job_id,
            UpdateProofJobStatusRequest {
                next_status: JobStatus::Failed,
                error_code: Some(code),
                error_message: Some(message),
            },
        )
        .await;
        let _: usize = conn
            .lpush(DLQ_KEY, job_id)
            .await
            .map_err(|e| format!("dead-letter push failed: {e}"))?;
        release_processing_entry(&mut conn, job_id).await?;
        return Ok(());
    }

    let max_retries = state.config.worker_max_retries.max(0);
    if attempts > max_retries {
        metrics_service::set_last_error_ts(now_unix());
        let _ = crud::update_proof_job_status(
            state,
            job_id,
            UpdateProofJobStatusRequest {
                next_status: JobStatus::Failed,
                error_code: Some("WORKER_RETRY_EXHAUSTED".to_string()),
                error_message: Some(reason.to_string()),
            },
        )
        .await;
        let _: usize = conn
            .lpush(DLQ_KEY, job_id)
            .await
            .map_err(|e| format!("dead-letter push failed: {e}"))?;
        release_processing_entry(&mut conn, job_id).await?;
        return Ok(());
    }

    let backoff = state.config.worker_backoff_base_seconds.max(1) * (1_i64 << (attempts - 1));
    let retry_at = now_unix() + backoff;
    let _: usize = redis::cmd("ZADD")
        .arg(RETRY_ZSET_KEY)
        .arg(retry_at)
        .arg(job_id)
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("retry schedule failed: {e}"))?;
    metrics_service::inc_retries_scheduled();
    release_processing_entry(&mut conn, job_id).await?;
    Ok(())
}

async fn promote_due_retries(state: &AppState) -> Result<(), String> {
    let Some(infra) = &state.infra else {
        return Ok(());
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect failed: {e}"))?;
    let now = now_unix();
    let due: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(RETRY_ZSET_KEY)
        .arg("-inf")
        .arg(now)
        .arg("LIMIT")
        .arg(0)
        .arg(50)
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("retry scan failed: {e}"))?;
    for job_id in due {
        let _: usize = redis::cmd("ZREM")
            .arg(RETRY_ZSET_KEY)
            .arg(&job_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| format!("retry zrem failed: {e}"))?;
        let _: usize = conn
            .lpush(QUEUE_KEY, &job_id)
            .await
            .map_err(|e| format!("retry requeue failed: {e}"))?;
    }
    Ok(())
}

async fn release_processing_entry(
    conn: &mut MultiplexedConnection,
    job_id: &str,
) -> Result<(), String> {
    let _: usize = conn
        .lrem(PROCESSING_KEY, 1, job_id)
        .await
        .map_err(|e| format!("processing cleanup failed: {e}"))?;
    Ok(())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as i64
}

fn parse_non_retryable(reason: &str) -> Option<(String, String)> {
    let rest = reason.strip_prefix("NON_RETRYABLE:")?;
    let mut parts = rest.splitn(2, ':');
    let code = parts.next()?.trim();
    let message = parts.next().unwrap_or("non-retryable worker error").trim();
    if code.is_empty() {
        return None;
    }
    Some((code.to_string(), message.to_string()))
}
