use axum::body::{Body, to_bytes};
use compliance_attestation_adapter::app::{AppState, build_router};
use compliance_attestation_adapter::config::environment::AppConfig;
use compliance_attestation_adapter::module::compliance_attestation::schema::{
    ComplianceDecision, IntakeComplianceRequest, IntakeComplianceResponse, SubjectInput, EntityInput
};
use http::Request;
use tower::util::ServiceExt;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use uuid::Uuid;

#[tokio::test]
async fn intake_normalizes_and_flags_sanctions_hit() {
    let sanctions_path = std::env::temp_dir().join(format!(
        "zkclear-sanctions-{}.json",
        Uuid::now_v7()
    ));
    fs::write(
        &sanctions_path,
        r#"[{"source":"TEST","program":"DEMO","name":"Acme Restricted Trading LLC","jurisdiction":"GB","address":null}]"#,
    )
    .expect("write sanctions fixture");

    let config = AppConfig {
        rust_env: "test".to_string(),
        api_host: "127.0.0.1".to_string(),
        api_port: 0,
        mongodb_url: None,
        mongodb_database: None,
        redis_url: None,
        intake_max_age_seconds: 300,
        intake_max_future_skew_seconds: 30,
        sanctions_data_path: sanctions_path.to_string_lossy().to_string(),
        policy_snapshot_path: "config/policy_snapshot.json".to_string(),
        policy_version: "policy-v1".to_string(),
        attestation_ttl_seconds: 3600,
        replay_ttl_seconds: 86400,
        require_internal_signature: false,
        internal_signing_secret: None,
        encryption_key_hex: None,
    };
    let app = build_router(AppState::new(config.clone(), None));

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64;

    let request = IntakeComplianceRequest {
        workflow_run_id: "run-1".to_string(),
        request_id: "req-1".to_string(),
        nonce: "nonce-1".to_string(),
        timestamp: now,
        internal_signature: None,
        subjects: vec![SubjectInput {
            counterparty: None,
            entity: Some(EntityInput {
                entity_id: "ent-001".to_string(),
                registration_country: Some("GB".to_string()),
                legal_name: Some("Acme Restricted Trading LLC".to_string()),
            }),
        }],
    };

    let http_req = Request::builder()
        .method("POST")
        .uri("/v1/compliance/intake")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&request).expect("serialize"),
        ))
        .expect("build request");

    let http_resp = app.oneshot(http_req).await.expect("response");
    assert_eq!(http_resp.status(), http::StatusCode::OK);

    let body = to_bytes(http_resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    let parsed: IntakeComplianceResponse = serde_json::from_slice(&body).expect("parse body");

    assert!(parsed.accepted);
    assert_eq!(parsed.decision.as_str(), ComplianceDecision::Fail.as_str());
    assert!(parsed.sanctions_hit_count > 0);
    assert!(!parsed.attestation_hash.is_empty());
    assert!(!parsed.attestation_id.is_empty());
    assert_eq!(parsed.issued_at, now);
    assert_eq!(parsed.expires_at, now + config.attestation_ttl_seconds);
    assert_eq!(parsed.normalized_subject_count, 1);

    let app2 = build_router(AppState::new(config, None));
    let http_req2 = Request::builder()
        .method("POST")
        .uri("/v1/compliance/intake")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&request).expect("serialize"),
        ))
        .expect("build request");
    let http_resp2 = app2.oneshot(http_req2).await.expect("response");
    let body2 = to_bytes(http_resp2.into_body(), usize::MAX)
        .await
        .expect("read body");
    let parsed2: IntakeComplianceResponse = serde_json::from_slice(&body2).expect("parse body");
    assert_eq!(parsed.attestation_hash, parsed2.attestation_hash);
    assert_eq!(parsed.attestation_id, parsed2.attestation_id);
}
