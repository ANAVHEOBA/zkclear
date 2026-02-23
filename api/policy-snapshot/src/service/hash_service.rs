use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn hmac_sha256_hex(input: &str, secret: &str) -> Result<String, String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("hmac init failed: {e}"))?;
    mac.update(input.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}
