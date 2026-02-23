use super::model::{ActivePolicyRecord, PolicySnapshotRecord, RunPolicyEvidenceRecord};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateSnapshotRequest {
    pub policy_version: String,
    pub policy_hash: Option<String>,
    pub rules: Value,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateSnapshotResponse {
    pub accepted: bool,
    pub idempotent: bool,
    pub policy_version: String,
    pub policy_hash: String,
    pub canonical_rules_json: String,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActivatePolicyRequest {
    pub onchain_policy_version: String,
    pub policy_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActivatePolicyResponse {
    pub accepted: bool,
    pub active_mapping: ActivePolicyRecord,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnapshotLookupResponse {
    pub found: bool,
    pub snapshot: Option<PolicySnapshotRecord>,
    pub active_mapping: Option<ActivePolicyRecord>,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActivePolicyResponse {
    pub found: bool,
    pub active_mapping: Option<ActivePolicyRecord>,
    pub snapshot: Option<PolicySnapshotRecord>,
    pub error_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EffectivePolicyQuery {
    pub timestamp: i64,
    pub version_hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EffectivePolicyResponse {
    pub found: bool,
    pub run_id: String,
    pub snapshot: Option<PolicySnapshotRecord>,
    pub activation: Option<ActivePolicyRecord>,
    pub evidence: Option<RunPolicyEvidenceRecord>,
    pub error_code: Option<String>,
    pub reason: String,
}
