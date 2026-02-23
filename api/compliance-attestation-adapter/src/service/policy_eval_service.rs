use crate::module::compliance_attestation::schema::ComplianceDecision;
use crate::service::sanctions_service::ScreeningResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySnapshot {
    pub active: ActivePolicy,
    pub thresholds: PolicyThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePolicy {
    pub version: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyThresholds {
    pub review_confidence: u8,
    pub fail_confidence: u8,
    pub pass_risk_score: u16,
    pub review_risk_score: u16,
    pub fail_risk_score: u16,
}

pub fn load_policy_snapshot(path: &str) -> Result<PolicySnapshot, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("failed to read policy snapshot: {e}"))?;
    serde_json::from_str::<PolicySnapshot>(&raw)
        .map_err(|e| format!("failed to parse policy snapshot: {e}"))
}

pub fn evaluate_intake_policy(
    screening: &ScreeningResult,
    thresholds: &PolicyThresholds,
) -> (ComplianceDecision, u16) {
    let max_conf = screening.hits.iter().map(|h| h.confidence).max().unwrap_or(0);
    if max_conf >= thresholds.fail_confidence {
        return (ComplianceDecision::Fail, thresholds.fail_risk_score);
    }
    if max_conf >= thresholds.review_confidence {
        return (ComplianceDecision::Review, thresholds.review_risk_score);
    }
    (ComplianceDecision::Pass, thresholds.pass_risk_score)
}
