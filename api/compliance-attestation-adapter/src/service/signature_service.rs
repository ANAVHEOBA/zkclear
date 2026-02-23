use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_internal_signature(
    payload: &str,
    signature_hex: &str,
    signing_secret: &str,
) -> Result<(), String> {
    let signature =
        hex::decode(signature_hex).map_err(|e| format!("invalid signature hex: {e}"))?;

    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())
        .map_err(|e| format!("hmac init failed: {e}"))?;
    mac.update(payload.as_bytes());

    mac.verify_slice(&signature)
        .map_err(|_| "signature verification failed".to_string())
}
