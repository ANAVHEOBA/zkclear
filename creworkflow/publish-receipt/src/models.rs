use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SettlementStatus {
    Accepted,
    Rejected,
    Settled,
    Failed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishReceiptRequest {
    pub settlement_registry: String,
    pub publisher_address: String,
    pub workflow_run_id: String,
    pub proof_hash: String,
    pub policy_version: u64,
    pub status: SettlementStatus,
    pub receipt_hash: String,
    pub proof_hex: String,
    pub public_signals: Vec<String>,
    pub chain_validation: ChainValidationState,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChainValidationState {
    pub authorized_publisher: bool,
    pub policy_active: bool,
    pub proof_valid: bool,
    pub signal_binding_valid: bool,
    pub duplicate_workflow_run: bool,
    pub duplicate_receipt_hash: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredReceiptRecord {
    pub workflow_run_id: String,
    pub proof_hash: String,
    pub policy_version: u64,
    pub status: SettlementStatus,
    pub receipt_hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublishReceiptResponse {
    pub settlement_registry: String,
    pub tx_hash: String,
    pub onchain_receipt_event_id: String,
    pub stored_receipt_record: StoredReceiptRecord,
}
