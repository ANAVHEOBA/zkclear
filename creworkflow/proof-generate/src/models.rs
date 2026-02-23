use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ProofGenerateRequest {
    pub workflow_run_id: String,
    pub match_result: MatchResult,
    pub policy_result: PolicyResult,
    pub settlement_params: SettlementParams,
    pub proving_timeout_ms: u64,
    pub estimated_proving_time_ms: u64,
    pub domain_separator: String,
    pub witness_seed: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchResult {
    pub accepted: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyResult {
    pub passed: bool,
    pub policy_version: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SettlementParams {
    pub asset_pair: String,
    pub buy_intent_id: String,
    pub sell_intent_id: String,
    pub execution_size: f64,
    pub execution_price: f64,
    pub notional: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProofGenerateResponse {
    pub proof_bytes: Vec<u8>,
    pub public_signals: Vec<String>,
    pub proof_hash: String,
    pub receipt_hash: String,
    pub policy_version: u64,
    pub domain_binding_hash: String,
}
