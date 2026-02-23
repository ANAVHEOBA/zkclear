use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProofType {
    Settlement,
    Compliance,
    Rebate,
}

impl ProofType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Settlement => "settlement",
            Self::Compliance => "compliance",
            Self::Rebate => "rebate",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    Queued,
    Proving,
    Proved,
    Publishing,
    Published,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "QUEUED",
            Self::Proving => "PROVING",
            Self::Proved => "PROVED",
            Self::Publishing => "PUBLISHING",
            Self::Published => "PUBLISHED",
            Self::Failed => "FAILED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitProofJobRequest {
    #[serde(alias = "workflowRunId")]
    pub workflow_run_id: String,
    #[serde(alias = "policyVersion")]
    pub policy_version: String,
    #[serde(alias = "receiptContext")]
    pub receipt_context: Value,
    #[serde(alias = "proofType")]
    pub proof_type: ProofType,
    #[serde(alias = "idempotencyKey")]
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitProofJobResponse {
    pub accepted: bool,
    pub idempotent: bool,
    pub replayed: bool,
    pub job_id: String,
    pub workflow_run_id: String,
    pub policy_version: String,
    pub proof_type: String,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusTransition {
    pub from_status: Option<JobStatus>,
    pub to_status: JobStatus,
    pub transitioned_at: i64,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverArtifactsView {
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
pub struct OnchainPublishView {
    pub settlement_registry: String,
    pub tx_hash: String,
    pub block_number: Option<u64>,
    pub onchain_status: String,
    pub onchain_receipt_event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofJobView {
    pub job_id: String,
    pub workflow_run_id: String,
    pub policy_version: String,
    pub proof_type: String,
    pub status: JobStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub prover_artifacts: Option<ProverArtifactsView>,
    pub onchain_publish: Option<OnchainPublishView>,
    pub transitions: Vec<JobStatusTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProofJobStatusRequest {
    pub next_status: JobStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProofJobStatusResponse {
    pub updated: bool,
    pub idempotent: bool,
    pub job: Option<ProofJobView>,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProofJobResponse {
    pub found: bool,
    pub job: Option<ProofJobView>,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatsResponse {
    pub available: bool,
    pub queued: u64,
    pub processing: u64,
    pub retry_scheduled: u64,
    pub dead_letter: u64,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProofJobsByRunResponse {
    pub found: bool,
    pub workflow_run_id: String,
    pub jobs: Vec<ProofJobView>,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryProofJobResponse {
    pub accepted: bool,
    pub job_id: String,
    pub status: Option<JobStatus>,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetricsView {
    pub jobs_queued: u64,
    pub jobs_published: u64,
    pub jobs_failed: u64,
    pub retries_scheduled: u64,
    pub prove_duration_count: u64,
    pub prove_duration_avg_ms: u64,
    pub queue_latency_count: u64,
    pub queue_latency_avg_ms: u64,
    pub last_error_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub redis_available: bool,
    pub mongo_available: bool,
    pub worker_enabled: bool,
    pub queue: QueueStatsResponse,
    pub metrics: HealthMetricsView,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationIntentRequest {
    pub encrypted_payload: String,
    pub signature: String,
    pub signer_public_key: String,
    pub nonce: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationCounterpartyInput {
    pub counterparty_id: String,
    pub country: Option<String>,
    pub wallet_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationEntityInput {
    pub entity_id: String,
    pub registration_country: Option<String>,
    pub legal_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationSubjectInput {
    pub counterparty: Option<OrchestrationCounterpartyInput>,
    pub entity: Option<OrchestrationEntityInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartOtcOrchestrationRequest {
    pub intents: Vec<OrchestrationIntentRequest>,
    pub subjects: Vec<OrchestrationSubjectInput>,
    pub proof_type: ProofType,
    pub idempotency_key: Option<String>,
    pub receipt_context: Option<Value>,
    pub request_id: Option<String>,
    pub compliance_nonce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtcIntentSubmitResult {
    pub accepted: bool,
    pub workflow_run_id: String,
    pub intent_ids: Vec<String>,
    pub commitment_hashes: Vec<String>,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartOtcOrchestrationResponse {
    pub accepted: bool,
    pub workflow_run_id: String,
    pub policy_version: String,
    pub attestation_id: String,
    pub attestation_hash: String,
    pub intent_submissions: Vec<OtcIntentSubmitResult>,
    pub proof_job: Option<SubmitProofJobResponse>,
    pub error_code: Option<String>,
    pub reason: String,
}
