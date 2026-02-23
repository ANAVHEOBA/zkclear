use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubmitIntentRequest {
    pub encrypted_payload: String,
    pub signature: String,
    pub signer_public_key: String,
    pub nonce: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitIntentResponse {
    pub workflow_run_id: String,
    pub intent_ids: Vec<String>,
    pub commitment_hashes: Vec<String>,
    pub accepted: bool,
    pub error_code: Option<String>,
    pub reason: String,
}
