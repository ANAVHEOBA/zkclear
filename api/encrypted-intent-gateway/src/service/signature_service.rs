use ed25519_dalek::{Signature, Verifier, VerifyingKey};

pub fn verify_signature(payload: &str, nonce: &str, timestamp: i64, signature_hex: &str, pubkey_hex: &str) -> Result<(), String> {
    let key_bytes = hex::decode(pubkey_hex).map_err(|e| format!("invalid signer_public_key hex: {e}"))?;
    let sig_bytes = hex::decode(signature_hex).map_err(|e| format!("invalid signature hex: {e}"))?;

    let key_arr: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "signer_public_key must be 32 bytes".to_string())?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "signature must be 64 bytes".to_string())?;

    let verifying_key = VerifyingKey::from_bytes(&key_arr).map_err(|e| format!("invalid public key: {e}"))?;
    let signature = Signature::from_bytes(&sig_arr);

    let message = format!("{payload}:{nonce}:{timestamp}");
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|e| format!("signature verification failed: {e}"))?;

    Ok(())
}
