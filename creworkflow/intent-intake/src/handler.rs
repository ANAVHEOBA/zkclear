use std::collections::HashSet;

use base64::Engine;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::IntakeError;
use crate::models::{
    DecryptedIntentPayload, EncryptedIntentEnvelope, IntentIntakeRequest, IntentIntakeResponse, NormalizedIntent,
};

pub fn process_intake(req: IntentIntakeRequest) -> Result<IntentIntakeResponse, IntakeError> {
    if req.encrypted_intents.len() != 2 {
        return Err(IntakeError::InvalidRequest(
            "exactly two encrypted intents are required".to_string(),
        ));
    }
    if req.payload_reference.trim().is_empty() {
        return Err(IntakeError::InvalidRequest(
            "payload_reference cannot be empty".to_string(),
        ));
    }

    let mut known_nonces: HashSet<String> = req.seen_nonces.into_iter().collect();
    let mut normalized_private_intents = Vec::with_capacity(2);
    let mut intent_commitment_hashes = Vec::with_capacity(2);

    for envelope in &req.encrypted_intents {
        let payload = decrypt_payload(envelope)?;
        validate_payload(envelope, &payload, req.current_unix_ts, &mut known_nonces)?;

        let normalized = NormalizedIntent {
            intent_id: envelope.intent_id.clone(),
            signer: envelope.signer.clone(),
            asset_pair: payload.asset_pair,
            side: payload.side,
            size: payload.size,
            limit_price: payload.limit_price,
            expiry: payload.expiry,
            counterparty_constraints: payload.counterparty_constraints,
            nonce: payload.nonce,
        };

        let commitment = compute_intent_commitment(&normalized)?;
        normalized_private_intents.push(normalized);
        intent_commitment_hashes.push(commitment);
    }

    Ok(IntentIntakeResponse {
        workflow_run_id: Uuid::now_v7().to_string(),
        payload_reference: req.payload_reference,
        normalized_private_intents,
        intent_commitment_hashes,
    })
}

fn decrypt_payload(envelope: &EncryptedIntentEnvelope) -> Result<DecryptedIntentPayload, IntakeError> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&envelope.ciphertext_b64)
        .map_err(|e| IntakeError::MalformedPayload {
            intent_id: envelope.intent_id.clone(),
            reason: format!("base64 decode failed: {e}"),
        })?;

    serde_json::from_slice::<DecryptedIntentPayload>(&decoded).map_err(|e| IntakeError::MalformedPayload {
        intent_id: envelope.intent_id.clone(),
        reason: format!("json decode failed: {e}"),
    })
}

fn validate_payload(
    envelope: &EncryptedIntentEnvelope,
    payload: &DecryptedIntentPayload,
    now: u64,
    known_nonces: &mut HashSet<String>,
) -> Result<(), IntakeError> {
    if payload.expiry <= now {
        return Err(IntakeError::ExpiredIntent {
            intent_id: envelope.intent_id.clone(),
            expiry: payload.expiry,
            now,
        });
    }

    if payload.issued_at > envelope.submitted_at || envelope.submitted_at > now + 300 {
        return Err(IntakeError::InvalidRequest(format!(
            "timestamp window invalid for intent {}",
            envelope.intent_id
        )));
    }

    if !known_nonces.insert(payload.nonce.clone()) {
        return Err(IntakeError::StaleNonce {
            intent_id: envelope.intent_id.clone(),
            nonce: payload.nonce.clone(),
        });
    }

    if !verify_signature(envelope, payload)? {
        return Err(IntakeError::InvalidSignature {
            intent_id: envelope.intent_id.clone(),
        });
    }

    Ok(())
}

fn verify_signature(envelope: &EncryptedIntentEnvelope, payload: &DecryptedIntentPayload) -> Result<bool, IntakeError> {
    let canonical = serde_json::to_string(payload).map_err(|e| IntakeError::MalformedPayload {
        intent_id: envelope.intent_id.clone(),
        reason: format!("canonical serialization failed: {e}"),
    })?;

    let mut hasher = Sha256::new();
    hasher.update(envelope.signer.as_bytes());
    hasher.update(canonical.as_bytes());
    let expected = hex::encode(hasher.finalize());

    Ok(expected.eq_ignore_ascii_case(&envelope.signature_hex))
}

fn compute_intent_commitment(intent: &NormalizedIntent) -> Result<String, IntakeError> {
    let canonical = serde_json::to_string(intent).map_err(|e| IntakeError::InvalidRequest(format!(
        "commitment serialization failed: {e}"
    )))?;
    let digest = Sha256::digest(canonical.as_bytes());
    Ok(format!("0x{}", hex::encode(digest)))
}
