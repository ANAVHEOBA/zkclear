use base64::Engine;
use intent_intake::errors::IntakeError;
use intent_intake::handler::process_intake;
use intent_intake::models::{
    CounterpartyConstraints, DecryptedIntentPayload, EncryptedIntentEnvelope, IntentIntakeRequest, Side,
};
use sha2::{Digest, Sha256};

fn sample_payload(nonce: &str, issued_at: u64, expiry: u64, side: Side) -> DecryptedIntentPayload {
    DecryptedIntentPayload {
        asset_pair: "ETH/USDC".to_string(),
        side,
        size: "100.5".to_string(),
        limit_price: "3025.10".to_string(),
        expiry,
        counterparty_constraints: CounterpartyConstraints {
            allow_list: vec!["desk_a".to_string()],
            deny_list: vec!["desk_x".to_string()],
        },
        nonce: nonce.to_string(),
        issued_at,
    }
}

fn make_envelope(intent_id: &str, signer: &str, submitted_at: u64, payload: &DecryptedIntentPayload) -> EncryptedIntentEnvelope {
    let canonical = serde_json::to_string(payload).expect("serialize payload");
    let mut hasher = Sha256::new();
    hasher.update(signer.as_bytes());
    hasher.update(canonical.as_bytes());
    let signature_hex = hex::encode(hasher.finalize());

    let ciphertext_b64 = base64::engine::general_purpose::STANDARD.encode(canonical.as_bytes());

    EncryptedIntentEnvelope {
        intent_id: intent_id.to_string(),
        signer: signer.to_string(),
        signature_hex,
        ciphertext_b64,
        submitted_at,
    }
}

#[test]
fn process_intake_success() {
    let now = 1_750_000_000_u64;
    let p1 = sample_payload("nonce-a", now - 10, now + 600, Side::Buy);
    let p2 = sample_payload("nonce-b", now - 8, now + 600, Side::Sell);

    let req = IntentIntakeRequest {
        encrypted_intents: vec![
            make_envelope("intent-1", "0x1111", now - 5, &p1),
            make_envelope("intent-2", "0x2222", now - 3, &p2),
        ],
        current_unix_ts: now,
        seen_nonces: vec![],
        payload_reference: "s3://bucket/ref-123".to_string(),
    };

    let out = process_intake(req).expect("intake succeeds");
    assert_eq!(out.normalized_private_intents.len(), 2);
    assert_eq!(out.intent_commitment_hashes.len(), 2);
    assert!(!out.workflow_run_id.is_empty());
    assert_eq!(out.payload_reference, "s3://bucket/ref-123");
}

#[test]
fn process_intake_fails_on_invalid_signature() {
    let now = 1_750_000_000_u64;
    let payload = sample_payload("nonce-a", now - 10, now + 600, Side::Buy);
    let mut bad = make_envelope("intent-1", "0x1111", now - 5, &payload);
    bad.signature_hex = "deadbeef".to_string();

    let req = IntentIntakeRequest {
        encrypted_intents: vec![bad, make_envelope("intent-2", "0x2222", now - 3, &payload)],
        current_unix_ts: now,
        seen_nonces: vec!["other".to_string()],
        payload_reference: "s3://bucket/ref-123".to_string(),
    };

    let err = process_intake(req).expect_err("must fail");
    assert!(matches!(err, IntakeError::InvalidSignature { .. }));
}

#[test]
fn process_intake_fails_on_stale_nonce() {
    let now = 1_750_000_000_u64;
    let payload = sample_payload("nonce-z", now - 10, now + 600, Side::Buy);

    let req = IntentIntakeRequest {
        encrypted_intents: vec![
            make_envelope("intent-1", "0x1111", now - 5, &payload),
            make_envelope("intent-2", "0x2222", now - 3, &payload),
        ],
        current_unix_ts: now,
        seen_nonces: vec!["nonce-z".to_string()],
        payload_reference: "s3://bucket/ref-123".to_string(),
    };

    let err = process_intake(req).expect_err("must fail");
    assert!(matches!(err, IntakeError::StaleNonce { .. }));
}

#[test]
fn process_intake_fails_on_expired_intent() {
    let now = 1_750_000_000_u64;
    let expired = sample_payload("nonce-a", now - 20, now - 1, Side::Buy);
    let valid = sample_payload("nonce-b", now - 10, now + 600, Side::Sell);

    let req = IntentIntakeRequest {
        encrypted_intents: vec![
            make_envelope("intent-1", "0x1111", now - 5, &expired),
            make_envelope("intent-2", "0x2222", now - 3, &valid),
        ],
        current_unix_ts: now,
        seen_nonces: vec![],
        payload_reference: "s3://bucket/ref-123".to_string(),
    };

    let err = process_intake(req).expect_err("must fail");
    assert!(matches!(err, IntakeError::ExpiredIntent { .. }));
}

#[test]
fn process_intake_fails_on_malformed_payload() {
    let now = 1_750_000_000_u64;
    let valid = sample_payload("nonce-a", now - 10, now + 600, Side::Buy);
    let mut bad = make_envelope("intent-1", "0x1111", now - 5, &valid);
    bad.ciphertext_b64 = "%%%not_base64%%%".to_string();

    let req = IntentIntakeRequest {
        encrypted_intents: vec![bad, make_envelope("intent-2", "0x2222", now - 3, &valid)],
        current_unix_ts: now,
        seen_nonces: vec![],
        payload_reference: "s3://bucket/ref-123".to_string(),
    };

    let err = process_intake(req).expect_err("must fail");
    assert!(matches!(err, IntakeError::MalformedPayload { .. }));
}
