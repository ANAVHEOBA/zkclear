use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct SettlePrivateRequest {
    pub workflow_run_id: String,
    pub proof_bundle: ProofBundle,
    pub settlement_instruction: SettlementInstruction,
    pub execution: ExecutionControl,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProofBundle {
    pub proof_hash: String,
    pub receipt_hash: String,
    pub approved: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SettlementInstruction {
    pub asset: String,
    pub amount: f64,
    pub from_account: String,
    pub to_account: String,
    pub transfer_simulation_ok: bool,
    pub counterparty_conflict: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionControl {
    pub max_retries: u32,
    pub timeout_ms: u64,
    pub estimated_execution_ms: u64,
    pub retryable_error_sequence: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SettlementStatus {
    Settled,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettlePrivateResponse {
    pub workflow_run_id: String,
    pub settlement_status: SettlementStatus,
    pub private_execution_reference_ids: Vec<String>,
    pub attempts_used: u32,
}
