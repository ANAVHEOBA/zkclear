use sha2::{Digest, Sha256};

pub fn verify_internal_signature(
    payload: &str,
    signature_hex: &str,
    secret: &str,
) -> Result<(), String> {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b":");
    hasher.update(payload.as_bytes());
    let expected = hex::encode(hasher.finalize());
    if expected.eq_ignore_ascii_case(signature_hex.trim()) {
        Ok(())
    } else {
        Err("signature verification failed".to_string())
    }
}
