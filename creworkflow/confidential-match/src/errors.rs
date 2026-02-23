use thiserror::Error;

#[derive(Debug, Error)]
pub enum MatchError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("compliance API unavailable")]
    ApiUnavailable,

    #[error("policy mismatch: expected={expected}, got={got}")]
    PolicyMismatch { expected: u64, got: u64 },

    #[error("risk threshold failed: risk_score={risk_score}, max={max}")]
    RiskThresholdFail { risk_score: u32, max: u32 },

    #[error("compliance check failed")]
    ComplianceFail,

    #[error("no match found")]
    NoMatch,
}
