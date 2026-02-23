use super::model::NormalizedSubject;
use crate::service::confidential_http_service::FxQuote;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntakeComplianceRequest {
    pub workflow_run_id: String,
    pub request_id: String,
    pub nonce: String,
    pub timestamp: i64,
    pub internal_signature: Option<String>,
    pub subjects: Vec<SubjectInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubjectInput {
    pub counterparty: Option<CounterpartyInput>,
    pub entity: Option<EntityInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CounterpartyInput {
    pub counterparty_id: String,
    pub country: Option<String>,
    pub wallet_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntityInput {
    pub entity_id: String,
    pub registration_country: Option<String>,
    pub legal_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntakeComplianceResponse {
    pub attestation_id: String,
    pub workflow_run_id: String,
    pub request_id: String,
    pub accepted: bool,
    pub normalized_subject_count: usize,
    pub normalized_subjects: Vec<NormalizedSubject>,
    pub policy_version: String,
    pub policy_hash: String,
    pub decision: ComplianceDecision,
    pub risk_score: u16,
    pub sanctions_hit_count: usize,
    pub attestation_hash: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub error_code: Option<String>,
    pub reason: String,
    pub fx_quote: Option<FxQuote>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub persistence_enabled: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComplianceDecision {
    Pass,
    Review,
    Fail,
}

impl ComplianceDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Review => "REVIEW",
            Self::Fail => "FAIL",
        }
    }
}
