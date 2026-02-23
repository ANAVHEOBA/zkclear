use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ConfidentialMatchRequest {
    pub workflow_run_id: String,
    pub policy: PolicyContext,
    pub intents: Vec<NormalizedIntent>,
    pub external_signals: ExternalSignals,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyContext {
    pub policy_version: u64,
    pub expected_policy_version: u64,
    pub max_risk_score: u32,
    pub max_notional: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExternalSignals {
    pub api_available: bool,
    pub compliance_passed: bool,
    pub risk_score: u32,
    pub attestation_payload: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NormalizedIntent {
    pub intent_id: String,
    pub signer: String,
    pub asset_pair: String,
    pub side: Side,
    pub size: f64,
    pub limit_price: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyCheckResult {
    pub passed: bool,
    pub policy_version: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettlementParams {
    pub asset_pair: String,
    pub buy_intent_id: String,
    pub sell_intent_id: String,
    pub execution_size: f64,
    pub execution_price: f64,
    pub notional: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfidentialMatchResponse {
    pub workflow_run_id: String,
    pub match_decision: MatchDecision,
    pub private_settlement_params: SettlementParams,
    pub policy_check_result: PolicyCheckResult,
    pub compliance_attestation_hash: String,
}
