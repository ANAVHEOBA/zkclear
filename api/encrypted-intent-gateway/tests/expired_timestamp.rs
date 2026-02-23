mod common;

use common::{
    build_test_context, encrypted_payload_b64, post_submit, sign_request, signer_public_key_hex,
    test_guard, unix_now,
};
use encrypted_intent_gateway::module::encrypted_intent::schema::SubmitIntentRequest;
use uuid::Uuid;

#[tokio::test]
async fn expired_timestamp_should_be_rejected() {
    let _guard = test_guard();
    let mut ctx = build_test_context().await;

    let timestamp = unix_now() - ctx.max_age_seconds - 5;
    let nonce = format!("nonce-expired-{}", Uuid::now_v7());
    let payload = encrypted_payload_b64(&ctx.decrypt_key_hex, "{\"asset\":\"ETH/USDC\",\"size\":10}");
    let signature = sign_request(&ctx.signing_key, &payload, &nonce, timestamp);

    let req = SubmitIntentRequest {
        encrypted_payload: payload,
        signature,
        signer_public_key: signer_public_key_hex(&ctx.signing_key),
        nonce: nonce.clone(),
        timestamp,
    };

    let (status, body) = post_submit(&mut ctx.app, &req).await;
    assert_eq!(status, http::StatusCode::BAD_REQUEST);
    assert!(!body.accepted);
    assert_eq!(body.error_code.as_deref(), Some("INTENT_EXPIRED"));
}
