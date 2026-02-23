use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

pub fn decrypt_intent(ciphertext_b64: &str) -> Result<String, String> {
    let is_confidential = load_confidential_runtime()?;
    let key = load_decryption_key()?;
    decrypt_intent_with(ciphertext_b64, is_confidential, &key)
}

fn decrypt_intent_with(
    ciphertext_b64: &str,
    is_confidential_runtime: bool,
    key: &[u8; KEY_LEN],
) -> Result<String, String> {
    if !is_confidential_runtime {
        return Err("decrypt blocked: not in confidential runtime".to_string());
    }

    let raw = STANDARD
        .decode(ciphertext_b64)
        .map_err(|e| format!("invalid encrypted_payload base64: {e}"))?;

    if raw.len() <= NONCE_LEN {
        return Err("encrypted_payload is too short".to_string());
    }

    let (nonce_bytes, cipher_bytes) = raw.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("invalid cipher key: {e}"))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plain = cipher
        .decrypt(nonce, cipher_bytes)
        .map_err(|_| "decrypt failed".to_string())?;

    String::from_utf8(plain).map_err(|e| format!("decrypted payload is not utf8: {e}"))
}

fn load_confidential_runtime() -> Result<bool, String> {
    let is_confidential = std::env::var("CONFIDENTIAL_RUNTIME").unwrap_or_default();
    if is_confidential.eq_ignore_ascii_case("true") || is_confidential == "1" {
        return Ok(true);
    }
    Ok(false)
}

fn load_decryption_key() -> Result<[u8; KEY_LEN], String> {
    let key_hex = std::env::var("INTENT_DECRYPTION_KEY_HEX")
        .map_err(|_| "missing INTENT_DECRYPTION_KEY_HEX".to_string())?;
    let key_bytes = hex::decode(key_hex).map_err(|e| format!("invalid key hex: {e}"))?;
    key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "INTENT_DECRYPTION_KEY_HEX must decode to 32 bytes".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::aead::Aead;

    fn make_ciphertext(plaintext: &str, key: &[u8; KEY_LEN], nonce: [u8; NONCE_LEN]) -> String {
        let cipher = Aes256Gcm::new_from_slice(key).expect("valid key");
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
            .expect("encrypt");
        let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        STANDARD.encode(out)
    }

    #[test]
    fn decrypt_fails_outside_confidential_runtime() {
        let key = [7u8; KEY_LEN];
        let ciphertext = make_ciphertext("{\"ok\":true}", &key, [1u8; NONCE_LEN]);
        let err = decrypt_intent_with(&ciphertext, false, &key).expect_err("must fail");
        assert!(err.contains("not in confidential runtime"));
    }

    #[test]
    fn decrypt_succeeds_in_confidential_runtime() {
        let key = [9u8; KEY_LEN];
        let plaintext = "{\"asset\":\"ETH/USDC\"}";
        let ciphertext = make_ciphertext(plaintext, &key, [2u8; NONCE_LEN]);
        let out = decrypt_intent_with(&ciphertext, true, &key).expect("must decrypt");
        assert_eq!(out, plaintext);
    }
}
