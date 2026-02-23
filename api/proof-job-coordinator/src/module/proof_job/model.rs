use super::schema::{JobStatus, JobStatusTransition, ProofType};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverArtifactsRecord {
    pub circuit: String,
    pub fixture_path: String,
    pub proof_path: String,
    pub public_path: String,
    pub proof_json: Value,
    pub public_json: Value,
    pub proof_hash: String,
    pub receipt_hash: String,
    pub prove_time_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainPublishRecord {
    pub settlement_registry: String,
    pub tx_hash: String,
    pub block_number: Option<u64>,
    pub onchain_status: String,
    pub onchain_receipt_event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofJobRecord {
    pub job_id: String,
    pub workflow_run_id: String,
    pub policy_version: String,
    pub proof_type: ProofType,
    pub receipt_context: Value,
    pub idempotency_key: String,
    pub request_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: JobStatus,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub prover_artifacts: Option<ProverArtifactsRecord>,
    pub onchain_publish: Option<OnchainPublishRecord>,
    pub transitions: Vec<JobStatusTransition>,
}

#[derive(Debug, Clone)]
pub struct IdempotencyRecord {
    pub request_hash: String,
    pub response: super::schema::SubmitProofJobResponse,
}
