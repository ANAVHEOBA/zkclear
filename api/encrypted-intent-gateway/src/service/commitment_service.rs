pub fn compute_commitment(
    payload: &str,
    nonce: &str,
    timestamp: i64,
    signer_public_key: &str,
) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    hasher.update(b"|");
    hasher.update(nonce.as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(signer_public_key.as_bytes());
    hex::encode(hasher.finalize())
}
