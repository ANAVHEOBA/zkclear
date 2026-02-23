use crate::service::confidential_http_service::FxQuote;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSubject {
    pub subject_id: String,
    pub subject_type: SubjectType,
    pub jurisdiction: Option<String>,
    pub address: Option<String>,
    pub legal_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubjectType {
    Counterparty,
    Entity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequestRecord {
    pub request_id: String,
    pub nonce: String,
    pub request_hash: String,
    pub workflow_run_id: String,
    pub received_at: i64,
    pub request_timestamp: i64,
    pub policy_version: String,
    pub policy_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponseReference {
    pub request_id: String,
    pub provider_ref_id: String,
    pub source: String,
    pub redacted_payload_ref: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceAttestationRecord {
    pub attestation_id: String,
    pub request_id: String,
    pub workflow_run_id: String,
    pub policy_version: String,
    pub policy_hash: String,
    pub decision: String,
    pub risk_score: u16,
    pub attestation_hash: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub sanctions_hit_count: usize,
    pub normalized_subjects: Vec<NormalizedSubject>,
    pub fx_quote: Option<FxQuote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventRecord {
    pub request_id: String,
    pub attestation_id: Option<String>,
    pub event_type: String,
    pub status: String,
    pub timestamp: i64,
    pub details: Option<String>,
}
