use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct IntentIntakeRequest {
    pub encrypted_intents: Vec<EncryptedIntentEnvelope>,
    pub current_unix_ts: u64,
    pub seen_nonces: Vec<String>,
    pub payload_reference: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EncryptedIntentEnvelope {
    pub intent_id: String,
    pub signer: String,
    pub signature_hex: String,
    pub ciphertext_b64: String,
    pub submitted_at: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DecryptedIntentPayload {
    pub asset_pair: String,
    pub side: Side,
    pub size: String,
    pub limit_price: String,
    pub expiry: u64,
    pub counterparty_constraints: CounterpartyConstraints,
    pub nonce: String,
    pub issued_at: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CounterpartyConstraints {
    pub allow_list: Vec<String>,
    pub deny_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedIntent {
    pub intent_id: String,
    pub signer: String,
    pub asset_pair: String,
    pub side: Side,
    pub size: String,
    pub limit_price: String,
    pub expiry: u64,
    pub counterparty_constraints: CounterpartyConstraints,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntentIntakeResponse {
    pub workflow_run_id: String,
    pub payload_reference: String,
    pub normalized_private_intents: Vec<NormalizedIntent>,
    pub intent_commitment_hashes: Vec<String>,
}
