use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use encrypted_intent_gateway::app::{AppState, build_router};
use encrypted_intent_gateway::config::db::{MongoConfig, RedisConfig};
use encrypted_intent_gateway::config::environment::AppConfig;
use encrypted_intent_gateway::infra::init_infra;
use encrypted_intent_gateway::module::encrypted_intent::schema::{
    SubmitIntentRequest, SubmitIntentResponse,
};
use http::Request;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::util::ServiceExt;
use uuid::Uuid;

pub static TEST_LOCK: Mutex<()> = Mutex::new(());

pub struct TestContext {
    pub app: axum::Router,
    pub signing_key: SigningKey,
    pub decrypt_key_hex: String,
    #[allow(dead_code)]
    pub max_age_seconds: i64,
}

pub async fn build_test_context() -> TestContext {
    load_test_env();
    let mut config = AppConfig::from_env().expect("missing env for integration tests");
    let suffix = Uuid::now_v7().simple().to_string();
    let short_suffix = &suffix[..8];
    config.mongodb_database = format!("zkit_{short_suffix}");

    let mongo = MongoConfig::from_app(&config);
    let redis = RedisConfig::from_app(&config);
    let infra = init_infra(&mongo, &redis)
        .await
        .expect("failed to initialize Mongo/Redis for integration tests");
    let app = build_router(AppState::new(config.clone(), infra));

    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let decrypt_key_hex =
        "1111111111111111111111111111111111111111111111111111111111111111".to_string();

    unsafe {
        std::env::set_var("CONFIDENTIAL_RUNTIME", "true");
        std::env::set_var("INTENT_DECRYPTION_KEY_HEX", &decrypt_key_hex);
    }

    TestContext {
        app,
        signing_key,
        decrypt_key_hex,
        max_age_seconds: config.intent_max_age_seconds,
    }
}

pub async fn post_submit(
    app: &mut axum::Router,
    req: &SubmitIntentRequest,
) -> (http::StatusCode, SubmitIntentResponse) {
    let request = Request::builder()
        .method("POST")
        .uri("/v1/intents/submit")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(req).expect("serialize request"),
        ))
        .expect("build request");

    let response = app.oneshot(request).await.expect("request failed");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: SubmitIntentResponse =
        serde_json::from_slice(&body).expect("deserialize response");
    (status, payload)
}

pub fn sign_request(
    signing_key: &SigningKey,
    payload: &str,
    nonce: &str,
    timestamp: i64,
) -> String {
    let msg = format!("{payload}:{nonce}:{timestamp}");
    let sig = signing_key.sign(msg.as_bytes());
    hex::encode(sig.to_bytes())
}

pub fn signer_public_key_hex(signing_key: &SigningKey) -> String {
    hex::encode(signing_key.verifying_key().to_bytes())
}

pub fn encrypted_payload_b64(key_hex: &str, plaintext: &str) -> String {
    let key_bytes = hex::decode(key_hex).expect("invalid key hex");
    let key_arr: [u8; 32] = key_bytes.as_slice().try_into().expect("invalid key size");
    let cipher = Aes256Gcm::new_from_slice(&key_arr).expect("cipher init");
    let nonce = [3u8; 12];
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .expect("encrypt");

    let mut out = Vec::with_capacity(nonce.len() + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    STANDARD.encode(out)
}

pub fn unix_now() -> i64 {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock error");
    elapsed.as_secs() as i64
}

pub fn test_guard() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().expect("test lock")
}

fn load_test_env() {
    for path in [".env", "../.env", "../../.env"] {
        let _ = dotenvy::from_path(path);
    }
}
