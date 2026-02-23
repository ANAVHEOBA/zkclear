use axum::body::{Body, to_bytes};
use http::Request;
use hmac::{Hmac, Mac};
use policy_snapshot::app::{AppState, build_router};
use policy_snapshot::config::environment::AppConfig;
use policy_snapshot::module::policy_snapshot::schema::{
    ActivatePolicyRequest, ActivatePolicyResponse, ActivePolicyResponse, CreateSnapshotRequest,
    CreateSnapshotResponse, EffectivePolicyResponse, SnapshotLookupResponse,
};
use sha2::Sha256;
use serde_json::json;
use tower::util::ServiceExt;
use tokio::time::{Duration, sleep};

type HmacSha256 = Hmac<Sha256>;

#[tokio::test]
async fn snapshot_registry_active_mapping_and_deterministic_retrieval() {
    let app = build_router(AppState::new(AppConfig {
        rust_env: "test".to_string(),
        api_host: "127.0.0.1".to_string(),
        api_port: 0,
        audit_signing_secret: Some("test-audit-secret".to_string()),
        mongodb_url: None,
        mongodb_database: None,
        redis_url: None,
        require_internal_auth: false,
        internal_auth_secret: None,
    }, None));

    let create_req = CreateSnapshotRequest {
        policy_version: "policy-v1".to_string(),
        policy_hash: None,
        rules: json!({
            "limits": {"max_notional": 1000000, "min_notional": 100},
            "countries": ["US", "GB"],
            "thresholds": {"fail_confidence": 90, "review_confidence": 70}
        }),
        metadata: Some(json!({"owner":"risk-team"})),
    };
    let create_resp = post_json::<CreateSnapshotRequest, CreateSnapshotResponse>(
        app.clone(),
        "/v1/policy/snapshots",
        &create_req,
    )
    .await;
    assert_eq!(create_resp.0, http::StatusCode::OK);
    assert!(create_resp.1.accepted);
    assert!(!create_resp.1.idempotent);
    let first_hash = create_resp.1.policy_hash.clone();
    let first_canonical = create_resp.1.canonical_rules_json.clone();

    let idem_resp = post_json::<CreateSnapshotRequest, CreateSnapshotResponse>(
        app.clone(),
        "/v1/policy/snapshots",
        &create_req,
    )
    .await;
    assert_eq!(idem_resp.0, http::StatusCode::OK);
    assert!(idem_resp.1.idempotent);
    assert_eq!(idem_resp.1.policy_hash, first_hash);
    assert_eq!(idem_resp.1.canonical_rules_json, first_canonical);

    let conflict_req = CreateSnapshotRequest {
        policy_version: "policy-v1".to_string(),
        policy_hash: None,
        rules: json!({
            "limits": {"max_notional": 999999, "min_notional": 50},
            "countries": ["US", "GB"],
            "thresholds": {"fail_confidence": 91, "review_confidence": 71}
        }),
        metadata: None,
    };
    let conflict_resp = post_json::<CreateSnapshotRequest, CreateSnapshotResponse>(
        app.clone(),
        "/v1/policy/snapshots",
        &conflict_req,
    )
    .await;
    assert_eq!(conflict_resp.0, http::StatusCode::CONFLICT);
    assert_eq!(
        conflict_resp.1.error_code.as_deref(),
        Some("IMMUTABLE_VERSION_CONFLICT")
    );

    let activate_req = ActivatePolicyRequest {
        onchain_policy_version: "onchain-42".to_string(),
        policy_version: "policy-v1".to_string(),
    };
    let activate_resp = post_json::<ActivatePolicyRequest, ActivatePolicyResponse>(
        app.clone(),
        "/v1/policy/activate",
        &activate_req,
    )
    .await;
    assert_eq!(activate_resp.0, http::StatusCode::OK);
    assert!(activate_resp.1.accepted);
    assert_eq!(activate_resp.1.active_mapping.policy_hash, first_hash);
    let first_activation_time = activate_resp.1.active_mapping.activated_at;

    sleep(Duration::from_secs(1)).await;

    let create_v2 = CreateSnapshotRequest {
        policy_version: "policy-v2".to_string(),
        policy_hash: None,
        rules: json!({
            "limits": {"max_notional": 2000000, "min_notional": 200},
            "countries": ["US", "GB", "CA"],
            "thresholds": {"fail_confidence": 92, "review_confidence": 75}
        }),
        metadata: Some(json!({"owner":"risk-team"})),
    };
    let create_v2_resp = post_json::<CreateSnapshotRequest, CreateSnapshotResponse>(
        app.clone(),
        "/v1/policy/snapshots",
        &create_v2,
    )
    .await;
    assert_eq!(create_v2_resp.0, http::StatusCode::OK);

    let activate_v2 = ActivatePolicyRequest {
        onchain_policy_version: "onchain-43".to_string(),
        policy_version: "policy-v2".to_string(),
    };
    let activate_v2_resp = post_json::<ActivatePolicyRequest, ActivatePolicyResponse>(
        app.clone(),
        "/v1/policy/activate",
        &activate_v2,
    )
    .await;
    assert_eq!(activate_v2_resp.0, http::StatusCode::OK);
    let second_activation_time = activate_v2_resp.1.active_mapping.activated_at;

    let by_version = get_json::<SnapshotLookupResponse>(app.clone(), "/v1/policy/snapshots/policy-v1").await;
    assert_eq!(by_version.0, http::StatusCode::OK);
    let snapshot = by_version.1.snapshot.expect("snapshot");
    assert_eq!(snapshot.policy_hash, first_hash);
    assert_eq!(snapshot.canonical_rules_json, first_canonical);

    let by_hash = get_json::<SnapshotLookupResponse>(
        app.clone(),
        &format!("/v1/policy/snapshots/hash/{first_hash}"),
    )
    .await;
    assert_eq!(by_hash.0, http::StatusCode::OK);
    assert!(by_hash.1.found);

    let active_at_first = get_json::<ActivePolicyResponse>(
        app.clone(),
        &format!("/v1/policy/active/at/{first_activation_time}"),
    )
    .await;
    assert_eq!(active_at_first.0, http::StatusCode::OK);
    assert_eq!(
        active_at_first
            .1
            .active_mapping
            .as_ref()
            .expect("active first")
            .policy_version,
        "policy-v1"
    );

    let active_at_second = get_json::<ActivePolicyResponse>(
        app.clone(),
        &format!("/v1/policy/active/at/{second_activation_time}"),
    )
    .await;
    assert_eq!(active_at_second.0, http::StatusCode::OK);
    assert_eq!(
        active_at_second
            .1
            .active_mapping
            .as_ref()
            .expect("active second")
            .policy_version,
        "policy-v2"
    );

    let effective_first = get_json::<EffectivePolicyResponse>(
        app.clone(),
        &format!(
            "/v1/policy/effective/run-xyz?timestamp={}&version_hint=policy-v1",
            first_activation_time
        ),
    )
    .await;
    assert_eq!(effective_first.0, http::StatusCode::OK);
    assert!(effective_first.1.found);
    assert_eq!(
        effective_first
            .1
            .snapshot
            .as_ref()
            .expect("snapshot")
            .policy_version,
        "policy-v1"
    );
    let evidence_hash = effective_first
        .1
        .evidence
        .as_ref()
        .expect("evidence")
        .evidence_hash
        .clone();

    let effective_replay = get_json::<EffectivePolicyResponse>(
        app.clone(),
        &format!(
            "/v1/policy/effective/run-xyz?timestamp={}&version_hint=policy-v1",
            first_activation_time
        ),
    )
    .await;
    assert_eq!(effective_replay.0, http::StatusCode::OK);
    assert_eq!(
        effective_replay
            .1
            .evidence
            .as_ref()
            .expect("evidence")
            .evidence_hash,
        evidence_hash
    );

    let effective_conflict = get_json::<EffectivePolicyResponse>(
        app,
        &format!(
            "/v1/policy/effective/run-xyz?timestamp={}&version_hint=policy-v2",
            second_activation_time
        ),
    )
    .await;
    assert_eq!(effective_conflict.0, http::StatusCode::CONFLICT);
    assert_eq!(
        effective_conflict.1.error_code.as_deref(),
        Some("RUN_EVIDENCE_CONFLICT")
    );
}

#[tokio::test]
async fn create_snapshot_rejects_policy_hash_mismatch() {
    let app = build_router(AppState::new(AppConfig {
        rust_env: "test".to_string(),
        api_host: "127.0.0.1".to_string(),
        api_port: 0,
        audit_signing_secret: None,
        mongodb_url: None,
        mongodb_database: None,
        redis_url: None,
        require_internal_auth: false,
        internal_auth_secret: None,
    }, None));

    let req = CreateSnapshotRequest {
        policy_version: "policy-hash-mismatch".to_string(),
        policy_hash: Some("deadbeef".to_string()),
        rules: json!({
            "limits": {"max_notional": 10_000, "min_notional": 10},
            "countries": ["US"],
            "thresholds": {"fail_confidence": 90, "review_confidence": 70}
        }),
        metadata: None,
    };

    let resp =
        post_json::<CreateSnapshotRequest, CreateSnapshotResponse>(app, "/v1/policy/snapshots", &req).await;
    assert_eq!(resp.0, http::StatusCode::BAD_REQUEST);
    assert_eq!(resp.1.error_code.as_deref(), Some("POLICY_HASH_MISMATCH"));
}

#[tokio::test]
async fn create_snapshot_requires_internal_signature_when_enabled() {
    let secret = "internal-secret";
    let app = build_router(AppState::new(AppConfig {
        rust_env: "test".to_string(),
        api_host: "127.0.0.1".to_string(),
        api_port: 0,
        audit_signing_secret: None,
        mongodb_url: None,
        mongodb_database: None,
        redis_url: None,
        require_internal_auth: true,
        internal_auth_secret: Some(secret.to_string()),
    }, None));

    let req = CreateSnapshotRequest {
        policy_version: "policy-auth".to_string(),
        policy_hash: None,
        rules: json!({
            "limits": {"max_notional": 10_000, "min_notional": 10},
            "countries": ["US"],
            "thresholds": {"fail_confidence": 90, "review_confidence": 70}
        }),
        metadata: None,
    };

    let missing_sig =
        post_json::<CreateSnapshotRequest, CreateSnapshotResponse>(app.clone(), "/v1/policy/snapshots", &req)
            .await;
    assert_eq!(missing_sig.0, http::StatusCode::BAD_REQUEST);
    assert_eq!(missing_sig.1.error_code.as_deref(), Some("MISSING_SIGNATURE"));

    let payload = serde_json::to_string(&req).expect("serialize req");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac init");
    mac.update(payload.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let ok = post_json_with_signature::<CreateSnapshotRequest, CreateSnapshotResponse>(
        app,
        "/v1/policy/snapshots",
        &req,
        &signature,
    )
    .await;
    assert_eq!(ok.0, http::StatusCode::OK);
    assert!(ok.1.accepted);
}

async fn post_json<TReq: serde::Serialize, TResp: serde::de::DeserializeOwned>(
    app: axum::Router,
    path: &str,
    req: &TReq,
) -> (http::StatusCode, TResp) {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(req).expect("serialize")))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: TResp = serde_json::from_slice(&body).expect("parse body");
    (status, payload)
}

async fn post_json_with_signature<TReq: serde::Serialize, TResp: serde::de::DeserializeOwned>(
    app: axum::Router,
    path: &str,
    req: &TReq,
    signature: &str,
) -> (http::StatusCode, TResp) {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .header("x-internal-signature", signature)
        .body(Body::from(serde_json::to_vec(req).expect("serialize")))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: TResp = serde_json::from_slice(&body).expect("parse body");
    (status, payload)
}

async fn get_json<TResp: serde::de::DeserializeOwned>(
    app: axum::Router,
    path: &str,
) -> (http::StatusCode, TResp) {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: TResp = serde_json::from_slice(&body).expect("parse body");
    (status, payload)
}
