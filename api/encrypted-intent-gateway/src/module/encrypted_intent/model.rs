use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedIntent {
    pub intent_id: String,
    pub workflow_run_id: String,
    pub encrypted_payload: String,
    pub commitment_hash: String,
    pub signer_public_key: String,
    pub nonce: String,
    pub timestamp: i64,
}
