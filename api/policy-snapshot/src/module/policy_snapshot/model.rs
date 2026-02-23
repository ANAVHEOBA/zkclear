use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySnapshotRecord {
    pub policy_version: String,
    pub policy_hash: String,
    pub canonical_rules: Value,
    pub canonical_rules_json: String,
    pub metadata: Option<Value>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePolicyRecord {
    pub onchain_policy_version: String,
    pub policy_version: String,
    pub policy_hash: String,
    pub activated_at: i64,
    pub deactivated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunPolicyEvidenceRecord {
    pub run_id: String,
    pub run_timestamp: i64,
    pub version_hint: Option<String>,
    pub policy_version: String,
    pub policy_hash: String,
    pub activated_at: i64,
    pub deactivated_at: Option<i64>,
    pub evidence_hash: String,
    pub evidence_signature: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRecord {
    pub event_type: String,
    pub policy_version: Option<String>,
    pub policy_hash: Option<String>,
    pub run_id: Option<String>,
    pub timestamp: i64,
    pub details: Option<String>,
}
