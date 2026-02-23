mod common;

use common::{
    build_test_context, encrypted_payload_b64, post_submit, sign_request, signer_public_key_hex,
    test_guard, unix_now,
};
use encrypted_intent_gateway::module::encrypted_intent::schema::SubmitIntentRequest;
use uuid::Uuid;

#[tokio::test]
async fn replay_nonce_or_hash_should_be_rejected() {
    let _guard = test_guard();
    let mut ctx = build_test_context().await;

    let timestamp = unix_now();
    let nonce = format!("nonce-replay-{}", Uuid::now_v7());
    let payload_1 =
        encrypted_payload_b64(&ctx.decrypt_key_hex, "{\"asset\":\"ETH/USDC\",\"size\":10}");
    let sig_1 = sign_request(&ctx.signing_key, &payload_1, &nonce, timestamp);

    let req_1 = SubmitIntentRequest {
        encrypted_payload: payload_1,
        signature: sig_1,
        signer_public_key: signer_public_key_hex(&ctx.signing_key),
        nonce: nonce.clone(),
        timestamp,
    };

    let (status_1, body_1) = post_submit(&mut ctx.app, &req_1).await;
    assert_eq!(status_1, http::StatusCode::OK);
    assert!(body_1.accepted);

    let payload_2 =
        encrypted_payload_b64(&ctx.decrypt_key_hex, "{\"asset\":\"ETH/USDC\",\"size\":11}");
    let sig_2 = sign_request(&ctx.signing_key, &payload_2, &nonce, timestamp);
    let req_2 = SubmitIntentRequest {
        encrypted_payload: payload_2,
        signature: sig_2,
        signer_public_key: signer_public_key_hex(&ctx.signing_key),
        nonce: nonce.clone(),
        timestamp,
    };

    let (status_2, body_2) = post_submit(&mut ctx.app, &req_2).await;
    assert_eq!(status_2, http::StatusCode::BAD_REQUEST);
    assert!(!body_2.accepted);
    assert_eq!(body_2.error_code.as_deref(), Some("REPLAY_NONCE"));
}
