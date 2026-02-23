use thiserror::Error;

#[derive(Debug, Error)]
pub enum IntakeError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("malformed payload for intent {intent_id}: {reason}")]
    MalformedPayload { intent_id: String, reason: String },

    #[error("invalid signature for intent {intent_id}")]
    InvalidSignature { intent_id: String },

    #[error("stale nonce for intent {intent_id}: {nonce}")]
    StaleNonce { intent_id: String, nonce: String },

    #[error("expired intent {intent_id}: expiry={expiry}, now={now}")]
    ExpiredIntent {
        intent_id: String,
        expiry: u64,
        now: u64,
    },
}
