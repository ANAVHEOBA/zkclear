use axum::body::{Body, to_bytes};
use http::Request;
use proof_job_coordinator::app::{AppState, build_router};
use proof_job_coordinator::config::environment::AppConfig;
use proof_job_coordinator::module::proof_job::schema::{
    GetProofJobResponse, GetProofJobsByRunResponse, HealthResponse, JobStatus, ProofType,
    QueueStatsResponse, RetryProofJobResponse, SubmitProofJobRequest, SubmitProofJobResponse,
    UpdateProofJobStatusRequest, UpdateProofJobStatusResponse,
};
use serde_json::json;
use tower::util::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        rust_env: "test".to_string(),
        api_host: "127.0.0.1".to_string(),
        api_port: 0,
        mongodb_url: None,
        mongodb_database: None,
        redis_url: None,
        idempotency_ttl_seconds: 3600,
        worker_enabled: false,
        worker_poll_seconds: 1,
        worker_lease_seconds: 10,
        worker_max_retries: 3,
        worker_backoff_base_seconds: 1,
        zk_root_dir: "../../zk".to_string(),
        prove_timeout_seconds: 30,
        prove_budget_settlement_seconds: 15,
        prove_budget_compliance_seconds: 10,
        prove_budget_rebate_seconds: 10,
        signal_domain_separator: "zkclear:v1".to_string(),
        eth_sepolia_rpc_url: None,
        private_key: None,
        eth_sepolia_chain_id: 11155111,
        publish_settlement_registry: None,
        publish_publisher_address: None,
        internal_auth_enabled: false,
        internal_auth_secret: None,
        intent_gateway_base_url: "http://127.0.0.1:8080".to_string(),
        compliance_adapter_base_url: "http://127.0.0.1:8082".to_string(),
        policy_snapshot_base_url: "http://127.0.0.1:8083".to_string(),
    }
}

#[tokio::test]
async fn submit_proof_job_accepts_valid_payload() {
    let app = build_router(AppState::new(test_config(), None));
    let req = SubmitProofJobRequest {
        workflow_run_id: "run-1".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0xabc"}),
        proof_type: ProofType::Settlement,
        idempotency_key: "idem-1".to_string(),
    };

    let (status, body) = post_json(app, &req).await;
    assert_eq!(status, http::StatusCode::OK);
    assert!(body.accepted);
    assert!(!body.idempotent);
}

#[tokio::test]
async fn submit_proof_job_is_idempotent_for_same_key_and_payload() {
    let app = build_router(AppState::new(test_config(), None));
    let req = SubmitProofJobRequest {
        workflow_run_id: "run-2".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0xdef"}),
        proof_type: ProofType::Compliance,
        idempotency_key: "idem-2".to_string(),
    };

    let first = post_json(app.clone(), &req).await;
    let second = post_json(app, &req).await;

    assert_eq!(first.0, http::StatusCode::OK);
    assert_eq!(second.0, http::StatusCode::OK);
    assert_eq!(first.1.job_id, second.1.job_id);
    assert!(second.1.idempotent);
}

#[tokio::test]
async fn submit_proof_job_rejects_idempotency_conflict() {
    let app = build_router(AppState::new(test_config(), None));
    let req1 = SubmitProofJobRequest {
        workflow_run_id: "run-3".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0x111"}),
        proof_type: ProofType::Rebate,
        idempotency_key: "idem-3".to_string(),
    };
    let req2 = SubmitProofJobRequest {
        workflow_run_id: "run-3b".to_string(),
        policy_version: "policy-v2".to_string(),
        receipt_context: json!({"receiptHash":"0x222"}),
        proof_type: ProofType::Rebate,
        idempotency_key: "idem-3".to_string(),
    };

    let _ = post_json(app.clone(), &req1).await;
    let second = post_json(app, &req2).await;

    assert_eq!(second.0, http::StatusCode::CONFLICT);
    assert_eq!(second.1.error_code.as_deref(), Some("IDEMPOTENCY_CONFLICT"));
}

#[tokio::test]
async fn submit_proof_job_rejects_replay_by_run_and_type() {
    let app = build_router(AppState::new(test_config(), None));
    let req1 = SubmitProofJobRequest {
        workflow_run_id: "run-4".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0x333"}),
        proof_type: ProofType::Settlement,
        idempotency_key: "idem-4-a".to_string(),
    };
    let req2 = SubmitProofJobRequest {
        workflow_run_id: "run-4".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0x444"}),
        proof_type: ProofType::Settlement,
        idempotency_key: "idem-4-b".to_string(),
    };

    let _ = post_json(app.clone(), &req1).await;
    let second = post_json(app, &req2).await;

    assert_eq!(second.0, http::StatusCode::CONFLICT);
    assert_eq!(
        second.1.error_code.as_deref(),
        Some("REPLAY_RUN_PROOF_TYPE")
    );
}

#[tokio::test]
async fn status_machine_allows_happy_path_transitions() {
    let app = build_router(AppState::new(test_config(), None));
    let req = SubmitProofJobRequest {
        workflow_run_id: "run-state-1".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0xaaa"}),
        proof_type: ProofType::Settlement,
        idempotency_key: "idem-state-1".to_string(),
    };
    let (_, created) = post_json(app.clone(), &req).await;

    let (_, proving) = post_status(
        app.clone(),
        &created.job_id,
        &UpdateProofJobStatusRequest {
            next_status: JobStatus::Proving,
            error_code: None,
            error_message: None,
        },
    )
    .await;
    assert_eq!(
        proving.job.expect("job").status.as_str(),
        JobStatus::Proving.as_str()
    );

    let _ = post_status(
        app.clone(),
        &created.job_id,
        &UpdateProofJobStatusRequest {
            next_status: JobStatus::Proved,
            error_code: None,
            error_message: None,
        },
    )
    .await;
    let _ = post_status(
        app.clone(),
        &created.job_id,
        &UpdateProofJobStatusRequest {
            next_status: JobStatus::Publishing,
            error_code: None,
            error_message: None,
        },
    )
    .await;
    let _ = post_status(
        app.clone(),
        &created.job_id,
        &UpdateProofJobStatusRequest {
            next_status: JobStatus::Published,
            error_code: None,
            error_message: None,
        },
    )
    .await;

    let (status, found) = get_job(app, &created.job_id).await;
    assert_eq!(status, http::StatusCode::OK);
    let job = found.job.expect("job");
    assert_eq!(job.status.as_str(), JobStatus::Published.as_str());
    assert_eq!(job.transitions.len(), 5);
}

#[tokio::test]
async fn status_machine_rejects_invalid_transition() {
    let app = build_router(AppState::new(test_config(), None));
    let req = SubmitProofJobRequest {
        workflow_run_id: "run-state-2".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0xbbb"}),
        proof_type: ProofType::Settlement,
        idempotency_key: "idem-state-2".to_string(),
    };
    let (_, created) = post_json(app.clone(), &req).await;

    let (status, resp) = post_status(
        app,
        &created.job_id,
        &UpdateProofJobStatusRequest {
            next_status: JobStatus::Published,
            error_code: None,
            error_message: None,
        },
    )
    .await;

    assert_eq!(status, http::StatusCode::CONFLICT);
    assert_eq!(resp.error_code.as_deref(), Some("INVALID_STATE_TRANSITION"));
}

#[tokio::test]
async fn status_machine_requires_error_code_for_failed() {
    let app = build_router(AppState::new(test_config(), None));
    let req = SubmitProofJobRequest {
        workflow_run_id: "run-state-3".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0xccc"}),
        proof_type: ProofType::Settlement,
        idempotency_key: "idem-state-3".to_string(),
    };
    let (_, created) = post_json(app.clone(), &req).await;
    let _ = post_status(
        app.clone(),
        &created.job_id,
        &UpdateProofJobStatusRequest {
            next_status: JobStatus::Proving,
            error_code: None,
            error_message: None,
        },
    )
    .await;

    let (status, resp) = post_status(
        app,
        &created.job_id,
        &UpdateProofJobStatusRequest {
            next_status: JobStatus::Failed,
            error_code: None,
            error_message: Some("timeout".to_string()),
        },
    )
    .await;
    assert_eq!(status, http::StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_code.as_deref(),
        Some("FAILED_STATUS_REQUIRES_ERROR_CODE")
    );
}

#[tokio::test]
async fn queue_stats_reports_unavailable_without_redis() {
    let app = build_router(AppState::new(test_config(), None));
    let request = Request::builder()
        .method("GET")
        .uri("/v1/proof-jobs/queue-stats")
        .body(Body::empty())
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: QueueStatsResponse = serde_json::from_slice(&body).expect("parse body");
    assert_eq!(status, http::StatusCode::OK);
    assert!(!payload.available);
    assert_eq!(payload.error_code.as_deref(), Some("QUEUE_UNAVAILABLE"));
}

#[tokio::test]
async fn get_proof_jobs_by_run_returns_created_job() {
    let app = build_router(AppState::new(test_config(), None));
    let req = SubmitProofJobRequest {
        workflow_run_id: "run-by-lookup".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0xrun"}),
        proof_type: ProofType::Settlement,
        idempotency_key: "idem-run-lookup".to_string(),
    };
    let _ = post_json(app.clone(), &req).await;
    let (status, body) = get_jobs_by_run(app, "run-by-lookup").await;
    assert_eq!(status, http::StatusCode::OK);
    assert!(body.found);
    assert_eq!(body.jobs.len(), 1);
}

#[tokio::test]
async fn retry_endpoint_requeues_non_published_job() {
    let app = build_router(AppState::new(test_config(), None));
    let req = SubmitProofJobRequest {
        workflow_run_id: "run-retry".to_string(),
        policy_version: "policy-v1".to_string(),
        receipt_context: json!({"receiptHash":"0xretry"}),
        proof_type: ProofType::Settlement,
        idempotency_key: "idem-retry".to_string(),
    };
    let (_, created) = post_json(app.clone(), &req).await;
    let (status, body) = retry_job(app, &created.job_id).await;
    assert_eq!(status, http::StatusCode::OK);
    assert!(body.accepted);
    assert_eq!(body.job_id, created.job_id);
}

#[tokio::test]
async fn health_endpoint_reports_shape() {
    let app = build_router(AppState::new(test_config(), None));
    let request = Request::builder()
        .method("GET")
        .uri("/v1/proof-jobs/health")
        .body(Body::empty())
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: HealthResponse = serde_json::from_slice(&body).expect("parse body");
    assert_eq!(status, http::StatusCode::OK);
    assert!(!payload.worker_enabled || payload.queue.available || !payload.ok);
}

async fn post_json(
    app: axum::Router,
    req: &SubmitProofJobRequest,
) -> (http::StatusCode, SubmitProofJobResponse) {
    let request = Request::builder()
        .method("POST")
        .uri("/v1/proof-jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(req).expect("serialize")))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: SubmitProofJobResponse = serde_json::from_slice(&body).expect("parse body");
    (status, payload)
}

async fn post_status(
    app: axum::Router,
    job_id: &str,
    req: &UpdateProofJobStatusRequest,
) -> (http::StatusCode, UpdateProofJobStatusResponse) {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/v1/proof-jobs/{job_id}/status"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(req).expect("serialize")))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: UpdateProofJobStatusResponse = serde_json::from_slice(&body).expect("parse body");
    (status, payload)
}

async fn get_job(app: axum::Router, job_id: &str) -> (http::StatusCode, GetProofJobResponse) {
    let request = Request::builder()
        .method("GET")
        .uri(format!("/v1/proof-jobs/{job_id}"))
        .body(Body::empty())
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: GetProofJobResponse = serde_json::from_slice(&body).expect("parse body");
    (status, payload)
}

async fn get_jobs_by_run(
    app: axum::Router,
    workflow_run_id: &str,
) -> (http::StatusCode, GetProofJobsByRunResponse) {
    let request = Request::builder()
        .method("GET")
        .uri(format!("/v1/proof-jobs/run/{workflow_run_id}"))
        .body(Body::empty())
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: GetProofJobsByRunResponse = serde_json::from_slice(&body).expect("parse body");
    (status, payload)
}

async fn retry_job(app: axum::Router, job_id: &str) -> (http::StatusCode, RetryProofJobResponse) {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/v1/proof-jobs/{job_id}/retry"))
        .body(Body::empty())
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: RetryProofJobResponse = serde_json::from_slice(&body).expect("parse body");
    (status, payload)
}
