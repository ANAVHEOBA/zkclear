use chrono::Utc;
use ethers::types::{Address, Signature};
use ethers::utils::hash_message;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

const JWT_ISSUER: &str = "zkclear-proof-job-coordinator";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletClaims {
    pub sub: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
    pub iss: String,
}

pub fn normalize_wallet_address(address: &str) -> Result<String, String> {
    let parsed: Address = address
        .parse()
        .map_err(|_| "invalid wallet address".to_string())?;
    Ok(format!("0x{}", hex::encode(parsed.as_bytes())))
}

pub fn build_login_message(wallet: &str, nonce: &str, issued_at: i64, chain_id: u64) -> String {
    format!(
        "ZK-Clear Wallet Login\nwallet:{wallet}\nnonce:{nonce}\nissued_at:{issued_at}\nchain_id:{chain_id}"
    )
}

pub fn verify_personal_sign(message: &str, signature_hex: &str) -> Result<String, String> {
    let sig: Signature = signature_hex
        .parse()
        .map_err(|_| "invalid signature format".to_string())?;
    let digest = hash_message(message);
    let recovered = sig
        .recover(digest)
        .map_err(|_| "failed to recover signer".to_string())?;
    Ok(format!("0x{}", hex::encode(recovered.as_bytes())))
}

pub fn resolve_wallet_role(wallet: &str, role_map: &str, default_role: &str) -> String {
    let wallet_lc = wallet.to_ascii_lowercase();
    for pair in role_map.split([',', ';', '\n']) {
        let entry = pair.trim();
        if entry.is_empty() {
            continue;
        }
        let mut parts = entry.splitn(2, ':');
        let mapped_wallet = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
        let mapped_role = parts.next().unwrap_or_default().trim();
        if !mapped_wallet.is_empty() && !mapped_role.is_empty() && mapped_wallet == wallet_lc {
            return mapped_role.to_string();
        }
    }
    default_role.to_string()
}

pub fn issue_access_token(
    wallet: &str,
    role: &str,
    jwt_secret: &str,
    ttl_seconds: i64,
) -> Result<(String, i64), String> {
    if ttl_seconds <= 0 {
        return Err("WALLET_JWT_TTL_SECONDS must be positive".to_string());
    }
    let iat = Utc::now().timestamp();
    let exp = iat
        .checked_add(ttl_seconds)
        .ok_or_else(|| "invalid jwt expiration".to_string())?;
    let claims = WalletClaims {
        sub: wallet.to_string(),
        role: role.to_string(),
        iat,
        exp,
        iss: JWT_ISSUER.to_string(),
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| format!("jwt issue failed: {e}"))?;
    Ok((token, exp))
}

pub fn verify_access_token(token: &str, jwt_secret: &str) -> Result<WalletClaims, String> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[JWT_ISSUER]);
    let data = decode::<WalletClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| format!("jwt verify failed: {e}"))?;
    Ok(data.claims)
}
