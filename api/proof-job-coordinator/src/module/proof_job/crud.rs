use super::error::AppError;
use super::model::{
    IdempotencyRecord, OnchainPublishRecord, ProofJobRecord, ProverArtifactsRecord,
};
use super::schema::{
    GetProofJobResponse, GetProofJobsByRunResponse, JobStatus, JobStatusTransition,
    OnchainPublishView, ProofJobView, ProverArtifactsView, RetryProofJobResponse,
    SubmitProofJobRequest, SubmitProofJobResponse, UpdateProofJobStatusRequest,
    UpdateProofJobStatusResponse,
};
use crate::app::AppState;
use crate::infra::{
    PROOF_JOB_ATTEMPTS_COLLECTION, PROOF_JOBS_COLLECTION, PROOF_OUTPUTS_COLLECTION,
    PUBLISH_RECEIPTS_COLLECTION,
};
use crate::service::hash_service::sha256_hex;
use crate::service::metrics_service;
use crate::service::queue_service;
use crate::service::replay_service::replay_run_key;
use crate::service::validation_service::validate_submit_request;
use crate::service::workflow_service::generate_job_id;
use chrono::Utc;
use redis::AsyncCommands;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Default)]
pub struct ProofJobStore {
    inner: Mutex<ProofJobStoreInner>,
}

#[derive(Debug, Default)]
struct ProofJobStoreInner {
    jobs_by_id: HashMap<String, ProofJobRecord>,
    idempotency_by_key: HashMap<String, IdempotencyRecord>,
    replay_guard_run_and_type: HashSet<String>,
    replay_guard_receipt_hash: HashSet<String>,
}

pub async fn submit_proof_job(
    state: &AppState,
    req: SubmitProofJobRequest,
) -> Result<SubmitProofJobResponse, AppError> {
    validate_submit_request(&req)?;

    let request_hash = hash_request(&req)?;
    let job = {
        let mut inner = lock_store(&state.store)?;

        if let Some(existing) = inner.idempotency_by_key.get(&req.idempotency_key) {
            if existing.request_hash != request_hash {
                return Err(AppError::conflict(
                    "IDEMPOTENCY_CONFLICT",
                    "idempotency_key already used with different payload",
                ));
            }

            let mut replayed = existing.response.clone();
            replayed.idempotent = true;
            replayed.replayed = false;
            return Ok(replayed);
        }

        let run_replay_key = replay_run_key(&req.workflow_run_id, req.proof_type.as_str());
        if inner.replay_guard_run_and_type.contains(&run_replay_key) {
            return Err(AppError::conflict(
                "REPLAY_RUN_PROOF_TYPE",
                "proof job already submitted for workflow_run_id + proof_type",
            ));
        }

        if let Some(receipt_hash) = extract_receipt_hash(&req) {
            if inner.replay_guard_receipt_hash.contains(&receipt_hash) {
                return Err(AppError::conflict(
                    "REPLAY_RECEIPT_HASH",
                    "proof job already submitted for receipt hash",
                ));
            }
            inner.replay_guard_receipt_hash.insert(receipt_hash);
        }

        let now = Utc::now().timestamp();
        let job = ProofJobRecord {
            job_id: generate_job_id(),
            workflow_run_id: req.workflow_run_id.clone(),
            policy_version: req.policy_version.clone(),
            proof_type: req.proof_type.clone(),
            receipt_context: req.receipt_context,
            idempotency_key: req.idempotency_key.clone(),
            request_hash,
            created_at: now,
            updated_at: now,
            status: JobStatus::Queued,
            last_error_code: None,
            last_error_message: None,
            prover_artifacts: None,
            onchain_publish: None,
            transitions: vec![JobStatusTransition {
                from_status: None,
                to_status: JobStatus::Queued,
                transitioned_at: now,
                error_code: None,
            }],
        };

        inner.replay_guard_run_and_type.insert(run_replay_key);
        inner.jobs_by_id.insert(job.job_id.clone(), job.clone());
        inner.idempotency_by_key.insert(
            req.idempotency_key,
            IdempotencyRecord {
                request_hash: job.request_hash.clone(),
                response: SubmitProofJobResponse {
                    accepted: true,
                    idempotent: false,
                    replayed: false,
                    job_id: job.job_id.clone(),
                    workflow_run_id: job.workflow_run_id.clone(),
                    policy_version: job.policy_version.clone(),
                    proof_type: job.proof_type.as_str().to_string(),
                    error_code: None,
                    reason: "proof job accepted and queued".to_string(),
                },
            },
        );
        job
    };

    persist_proof_job(state, &job).await?;
    persist_attempt(state, &job.job_id, None, JobStatus::Queued, None, None).await?;
    queue_service::enqueue_proof_job(state, &job.job_id)
        .await
        .map_err(|e| AppError::internal("QUEUE_ENQUEUE_FAILED", e))?;
    metrics_service::inc_jobs_queued();

    Ok(SubmitProofJobResponse {
        accepted: true,
        idempotent: false,
        replayed: false,
        job_id: job.job_id,
        workflow_run_id: job.workflow_run_id,
        policy_version: job.policy_version,
        proof_type: job.proof_type.as_str().to_string(),
        error_code: None,
        reason: "proof job accepted and queued".to_string(),
    })
}

pub async fn get_proof_job(
    state: &AppState,
    job_id: &str,
) -> Result<GetProofJobResponse, AppError> {
    if let Some(job) = get_local_job(state, job_id)? {
        return Ok(GetProofJobResponse {
            found: true,
            job: Some(to_view(&job)),
            error_code: None,
            reason: "proof job found".to_string(),
        });
    }
    if let Some(job) = load_job_from_redis(state, job_id).await? {
        warm_job_in_memory(state, &job)?;
        return Ok(GetProofJobResponse {
            found: true,
            job: Some(to_view(&job)),
            error_code: None,
            reason: "proof job found".to_string(),
        });
    }
    Ok(GetProofJobResponse {
        found: false,
        job: None,
        error_code: Some("JOB_NOT_FOUND".to_string()),
        reason: "proof job not found".to_string(),
    })
}

pub async fn get_proof_jobs_by_run(
    state: &AppState,
    workflow_run_id: &str,
) -> Result<GetProofJobsByRunResponse, AppError> {
    if workflow_run_id.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_WORKFLOW_RUN_ID",
            "workflow_run_id is required",
        ));
    }

    let mut by_id: HashMap<String, ProofJobRecord> = HashMap::new();
    {
        let inner = lock_store(&state.store)?;
        for job in inner.jobs_by_id.values() {
            if job.workflow_run_id == workflow_run_id {
                by_id.insert(job.job_id.clone(), job.clone());
            }
        }
    }

    if let Some(infra) = &state.infra {
        let mut conn = infra
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::internal("REDIS_CONNECT_FAILED", e.to_string()))?;
        let run_key = format!("{PROOF_JOBS_COLLECTION}:run:{workflow_run_id}");
        let job_ids: Vec<String> = conn
            .smembers(run_key)
            .await
            .map_err(|e| AppError::internal("REDIS_QUERY_FAILED", e.to_string()))?;
        for job_id in job_ids {
            if let Some(job) = load_job_from_redis(state, &job_id).await? {
                by_id.insert(job.job_id.clone(), job);
            }
        }
    }

    let mut jobs = by_id.into_values().map(|j| to_view(&j)).collect::<Vec<_>>();
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let found = !jobs.is_empty();
    Ok(GetProofJobsByRunResponse {
        found,
        workflow_run_id: workflow_run_id.to_string(),
        jobs,
        error_code: None,
        reason: if found {
            "proof jobs found".to_string()
        } else {
            "no proof jobs found for workflow_run_id".to_string()
        },
    })
}

pub async fn retry_proof_job(
    state: &AppState,
    job_id: &str,
) -> Result<RetryProofJobResponse, AppError> {
    let job = get_proof_job_record(state, job_id).await?;
    if matches!(job.status, JobStatus::Publishing | JobStatus::Published) {
        return Err(AppError::conflict(
            "RETRY_NOT_ALLOWED",
            "cannot retry job already publishing/published",
        ));
    }
    queue_service::enqueue_proof_job(state, job_id)
        .await
        .map_err(|e| AppError::internal("QUEUE_ENQUEUE_FAILED", e))?;
    metrics_service::inc_retries_scheduled();
    Ok(RetryProofJobResponse {
        accepted: true,
        job_id: job_id.to_string(),
        status: Some(job.status),
        error_code: None,
        reason: "proof job requeued".to_string(),
    })
}

pub async fn get_proof_job_record(
    state: &AppState,
    job_id: &str,
) -> Result<ProofJobRecord, AppError> {
    if let Some(job) = get_local_job(state, job_id)? {
        return Ok(job);
    }
    if let Some(job) = load_job_from_redis(state, job_id).await? {
        warm_job_in_memory(state, &job)?;
        return Ok(job);
    }
    Err(AppError::not_found("JOB_NOT_FOUND", "proof job not found"))
}

pub async fn set_prover_artifacts(
    state: &AppState,
    job_id: &str,
    artifacts: ProverArtifactsRecord,
) -> Result<(), AppError> {
    let job = {
        let mut inner = lock_store(&state.store)?;
        let job = inner
            .jobs_by_id
            .get_mut(job_id)
            .ok_or_else(|| AppError::not_found("JOB_NOT_FOUND", "proof job not found"))?;
        job.prover_artifacts = Some(artifacts.clone());
        job.updated_at = Utc::now().timestamp();
        job.clone()
    };
    persist_proof_job(state, &job).await?;
    persist_proof_output(state, job_id, &artifacts).await?;
    Ok(())
}

pub async fn set_onchain_publish_result(
    state: &AppState,
    job_id: &str,
    result: OnchainPublishRecord,
) -> Result<(), AppError> {
    let job = {
        let mut inner = lock_store(&state.store)?;
        let job = inner
            .jobs_by_id
            .get_mut(job_id)
            .ok_or_else(|| AppError::not_found("JOB_NOT_FOUND", "proof job not found"))?;
        job.onchain_publish = Some(result.clone());
        job.updated_at = Utc::now().timestamp();
        job.clone()
    };
    persist_proof_job(state, &job).await?;
    persist_publish_receipt(state, job_id, &result).await?;
    Ok(())
}

pub async fn update_proof_job_status(
    state: &AppState,
    job_id: &str,
    req: UpdateProofJobStatusRequest,
) -> Result<UpdateProofJobStatusResponse, AppError> {
    let (job, from_status, to_status, error_code, error_message) = {
        let mut inner = lock_store(&state.store)?;
        let job = inner
            .jobs_by_id
            .get_mut(job_id)
            .ok_or_else(|| AppError::not_found("JOB_NOT_FOUND", "proof job not found"))?;

        if req.next_status == job.status {
            let same_error_code = req.error_code == job.last_error_code;
            let same_error_message = req.error_message == job.last_error_message;
            if same_error_code && same_error_message {
                return Ok(UpdateProofJobStatusResponse {
                    updated: true,
                    idempotent: true,
                    job: Some(to_view(job)),
                    error_code: None,
                    reason: "status update is idempotent".to_string(),
                });
            }
        }

        if !is_valid_transition(&job.status, &req.next_status) {
            return Err(AppError::conflict(
                "INVALID_STATE_TRANSITION",
                format!(
                    "cannot transition from {} to {}",
                    job.status.as_str(),
                    req.next_status.as_str()
                ),
            ));
        }

        if req.next_status == JobStatus::Failed
            && req.error_code.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(AppError::bad_request(
                "FAILED_STATUS_REQUIRES_ERROR_CODE",
                "error_code is required when transitioning to FAILED",
            ));
        }

        let now = Utc::now().timestamp();
        let previous = job.status.clone();
        job.status = req.next_status.clone();
        job.updated_at = now;
        job.last_error_code = req.error_code.clone();
        job.last_error_message = req.error_message.clone();
        job.transitions.push(JobStatusTransition {
            from_status: Some(previous.clone()),
            to_status: req.next_status.clone(),
            transitioned_at: now,
            error_code: job.last_error_code.clone(),
        });
        (
            job.clone(),
            Some(previous),
            req.next_status,
            req.error_code,
            req.error_message,
        )
    };

    persist_proof_job(state, &job).await?;
    persist_attempt(
        state,
        job_id,
        from_status,
        to_status.clone(),
        error_code.clone(),
        error_message,
    )
    .await?;

    if to_status == JobStatus::Published {
        metrics_service::inc_jobs_published();
    }
    if to_status == JobStatus::Failed {
        metrics_service::inc_jobs_failed();
    }

    Ok(UpdateProofJobStatusResponse {
        updated: true,
        idempotent: false,
        job: Some(to_view(&job)),
        error_code: None,
        reason: "status updated".to_string(),
    })
}

fn extract_receipt_hash(req: &SubmitProofJobRequest) -> Option<String> {
    req.receipt_context
        .get("receiptHash")
        .or_else(|| req.receipt_context.get("receipt_hash"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn hash_request(req: &SubmitProofJobRequest) -> Result<String, AppError> {
    let payload = serde_json::to_string(req).map_err(|e| {
        AppError::internal(
            "SERIALIZATION_ERROR",
            format!("request serialize failed: {e}"),
        )
    })?;
    Ok(sha256_hex(&payload))
}

fn is_valid_transition(from: &JobStatus, to: &JobStatus) -> bool {
    matches!(
        (from, to),
        (JobStatus::Queued, JobStatus::Proving)
            | (JobStatus::Proving, JobStatus::Proved)
            | (JobStatus::Proving, JobStatus::Failed)
            | (JobStatus::Proved, JobStatus::Publishing)
            | (JobStatus::Publishing, JobStatus::Published)
            | (JobStatus::Publishing, JobStatus::Failed)
    )
}

fn to_view(job: &ProofJobRecord) -> ProofJobView {
    ProofJobView {
        job_id: job.job_id.clone(),
        workflow_run_id: job.workflow_run_id.clone(),
        policy_version: job.policy_version.clone(),
        proof_type: job.proof_type.as_str().to_string(),
        status: job.status.clone(),
        created_at: job.created_at,
        updated_at: job.updated_at,
        last_error_code: job.last_error_code.clone(),
        last_error_message: job.last_error_message.clone(),
        prover_artifacts: job.prover_artifacts.as_ref().map(|a| ProverArtifactsView {
            circuit: a.circuit.clone(),
            fixture_path: a.fixture_path.clone(),
            proof_path: a.proof_path.clone(),
            public_path: a.public_path.clone(),
            proof_json: a.proof_json.clone(),
            public_json: a.public_json.clone(),
            proof_hash: a.proof_hash.clone(),
            receipt_hash: a.receipt_hash.clone(),
            prove_time_seconds: a.prove_time_seconds,
        }),
        onchain_publish: job.onchain_publish.as_ref().map(|p| OnchainPublishView {
            settlement_registry: p.settlement_registry.clone(),
            tx_hash: p.tx_hash.clone(),
            block_number: p.block_number,
            onchain_status: p.onchain_status.clone(),
            onchain_receipt_event_id: p.onchain_receipt_event_id.clone(),
        }),
        transitions: job.transitions.clone(),
    }
}

fn lock_store(store: &ProofJobStore) -> Result<MutexGuard<'_, ProofJobStoreInner>, AppError> {
    store
        .inner
        .lock()
        .map_err(|_| AppError::internal("STORE_LOCK_ERROR", "proof job store lock poisoned"))
}

fn get_local_job(state: &AppState, job_id: &str) -> Result<Option<ProofJobRecord>, AppError> {
    let inner = lock_store(&state.store)?;
    Ok(inner.jobs_by_id.get(job_id).cloned())
}

fn warm_job_in_memory(state: &AppState, job: &ProofJobRecord) -> Result<(), AppError> {
    let mut inner = lock_store(&state.store)?;
    inner.jobs_by_id.insert(job.job_id.clone(), job.clone());
    inner.idempotency_by_key.insert(
        job.idempotency_key.clone(),
        IdempotencyRecord {
            request_hash: job.request_hash.clone(),
            response: SubmitProofJobResponse {
                accepted: true,
                idempotent: false,
                replayed: false,
                job_id: job.job_id.clone(),
                workflow_run_id: job.workflow_run_id.clone(),
                policy_version: job.policy_version.clone(),
                proof_type: job.proof_type.as_str().to_string(),
                error_code: None,
                reason: "proof job accepted and queued".to_string(),
            },
        },
    );
    inner.replay_guard_run_and_type.insert(replay_run_key(
        &job.workflow_run_id,
        job.proof_type.as_str(),
    ));
    if let Some(receipt_hash) = job
        .prover_artifacts
        .as_ref()
        .map(|a| a.receipt_hash.clone())
        .or_else(|| extract_receipt_hash_from_context(&job.receipt_context))
    {
        inner.replay_guard_receipt_hash.insert(receipt_hash);
    }
    Ok(())
}

async fn load_job_from_redis(
    state: &AppState,
    job_id: &str,
) -> Result<Option<ProofJobRecord>, AppError> {
    let Some(infra) = &state.infra else {
        return Ok(None);
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_CONNECT_FAILED", e.to_string()))?;
    let key = format!("{PROOF_JOBS_COLLECTION}:{job_id}");
    let raw: Option<String> = conn
        .get(key)
        .await
        .map_err(|e| AppError::internal("REDIS_QUERY_FAILED", e.to_string()))?;
    raw.map(|s| serde_json::from_str::<ProofJobRecord>(&s))
        .transpose()
        .map_err(|e| AppError::internal("REDIS_DECODE_FAILED", e.to_string()))
}

async fn persist_proof_job(state: &AppState, job: &ProofJobRecord) -> Result<(), AppError> {
    let Some(infra) = &state.infra else {
        return Ok(());
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_CONNECT_FAILED", e.to_string()))?;
    let key = format!("{PROOF_JOBS_COLLECTION}:{}", job.job_id);
    let payload = serde_json::to_string(job)
        .map_err(|e| AppError::internal("SERIALIZATION_ERROR", e.to_string()))?;
    let _: () = conn
        .set(key, payload)
        .await
        .map_err(|e| AppError::internal("REDIS_WRITE_FAILED", e.to_string()))?;
    let run_key = format!("{PROOF_JOBS_COLLECTION}:run:{}", job.workflow_run_id);
    let _: usize = conn
        .sadd(run_key, &job.job_id)
        .await
        .map_err(|e| AppError::internal("REDIS_WRITE_FAILED", e.to_string()))?;
    Ok(())
}

async fn persist_attempt(
    state: &AppState,
    job_id: &str,
    from_status: Option<JobStatus>,
    to_status: JobStatus,
    error_code: Option<String>,
    error_message: Option<String>,
) -> Result<(), AppError> {
    let Some(infra) = &state.infra else {
        return Ok(());
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_CONNECT_FAILED", e.to_string()))?;
    let now = Utc::now().timestamp();
    let from_status_str = from_status.as_ref().map(|s| s.as_str().to_string());
    let record = json!({
        "job_id": job_id,
        "from_status": from_status_str,
        "to_status": to_status.as_str(),
        "transitioned_at": now,
        "error_code": error_code,
        "error_message": error_message,
        "evidence_hash": sha256_hex(&format!("{job_id}:{now}:{:?}:{:?}", from_status, to_status)),
    });
    let key = format!("{PROOF_JOB_ATTEMPTS_COLLECTION}:{job_id}");
    let payload = serde_json::to_string(&record)
        .map_err(|e| AppError::internal("SERIALIZATION_ERROR", e.to_string()))?;
    let _: usize = conn
        .lpush(key, payload)
        .await
        .map_err(|e| AppError::internal("REDIS_WRITE_FAILED", e.to_string()))?;
    Ok(())
}

async fn persist_proof_output(
    state: &AppState,
    job_id: &str,
    artifacts: &ProverArtifactsRecord,
) -> Result<(), AppError> {
    let Some(infra) = &state.infra else {
        return Ok(());
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_CONNECT_FAILED", e.to_string()))?;
    let record = json!({
        "job_id": job_id,
        "circuit": artifacts.circuit,
        "fixture_path": artifacts.fixture_path,
        "proof_path": artifacts.proof_path,
        "public_path": artifacts.public_path,
        "proof_hash": artifacts.proof_hash,
        "receipt_hash": artifacts.receipt_hash,
        "prove_time_seconds": artifacts.prove_time_seconds,
        "created_at": Utc::now().timestamp(),
        "evidence_hash": sha256_hex(&format!("{job_id}:{}:{}", artifacts.proof_hash, artifacts.receipt_hash)),
    });
    let key = format!("{PROOF_OUTPUTS_COLLECTION}:{job_id}");
    let payload = serde_json::to_string(&record)
        .map_err(|e| AppError::internal("SERIALIZATION_ERROR", e.to_string()))?;
    let _: usize = conn
        .lpush(key, payload)
        .await
        .map_err(|e| AppError::internal("REDIS_WRITE_FAILED", e.to_string()))?;
    Ok(())
}

async fn persist_publish_receipt(
    state: &AppState,
    job_id: &str,
    result: &OnchainPublishRecord,
) -> Result<(), AppError> {
    let Some(infra) = &state.infra else {
        return Ok(());
    };
    let mut conn = infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_CONNECT_FAILED", e.to_string()))?;
    let record = json!({
        "job_id": job_id,
        "settlement_registry": result.settlement_registry,
        "tx_hash": result.tx_hash,
        "block_number": result.block_number,
        "onchain_status": result.onchain_status,
        "onchain_receipt_event_id": result.onchain_receipt_event_id,
        "created_at": Utc::now().timestamp(),
        "evidence_hash": sha256_hex(&format!("{job_id}:{}:{}", result.tx_hash, result.onchain_receipt_event_id)),
    });
    let key = format!("{PUBLISH_RECEIPTS_COLLECTION}:{job_id}");
    let payload = serde_json::to_string(&record)
        .map_err(|e| AppError::internal("SERIALIZATION_ERROR", e.to_string()))?;
    let _: usize = conn
        .lpush(key, payload)
        .await
        .map_err(|e| AppError::internal("REDIS_WRITE_FAILED", e.to_string()))?;
    Ok(())
}

fn extract_receipt_hash_from_context(ctx: &serde_json::Value) -> Option<String> {
    ctx.get("receiptHash")
        .or_else(|| ctx.get("receipt_hash"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}
