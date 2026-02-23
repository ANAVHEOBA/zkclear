use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub fn encrypt_for_storage(plaintext: &str, key_hex: &str) -> Result<String, String> {
    let key_bytes = hex::decode(key_hex).map_err(|e| format!("invalid ENCRYPTION_KEY_HEX: {e}"))?;
    let key: [u8; KEY_LEN] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "ENCRYPTION_KEY_HEX must decode to 32 bytes".to_string())?;

    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init failed: {e}"))?;
    let nonce = [7u8; NONCE_LEN];
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|_| "encryption failed".to_string())?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(format!("enc:v1:{}", STANDARD.encode(out)))
}
