use proof_job_coordinator::app::AppState;
use proof_job_coordinator::config::environment::AppConfig;
use proof_job_coordinator::module::proof_job::model::{ProofJobRecord, ProverArtifactsRecord};
use proof_job_coordinator::module::proof_job::schema::{JobStatus, JobStatusTransition, ProofType};
use proof_job_coordinator::service::binding_codec_service::settlement_binding_fields;
use proof_job_coordinator::service::signal_binding_service::validate_public_signal_binding;
use serde_json::json;

fn test_state() -> AppState {
    AppState::new(
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
            worker_max_retries: 1,
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
            wallet_auth_enabled: false,
            wallet_auth_nonce_ttl_seconds: 300,
            wallet_jwt_secret: None,
            wallet_jwt_ttl_seconds: 3600,
            wallet_role_map: String::new(),
            wallet_default_role: "dealer".to_string(),
            intent_gateway_base_url: "http://127.0.0.1:8080".to_string(),
            compliance_adapter_base_url: "http://127.0.0.1:8082".to_string(),
            policy_snapshot_base_url: "http://127.0.0.1:8083".to_string(),
        },
        None,
    )
}

fn job() -> ProofJobRecord {
    ProofJobRecord {
        job_id: "proofjob-1".to_string(),
        workflow_run_id: "run-123".to_string(),
        policy_version: "policy-v1".to_string(),
        proof_type: ProofType::Settlement,
        receipt_context: json!({
            "receiptHash":"receipt-999",
            "publicSignalIndex":{
                "policyVersion":0,
                "receiptHash":1,
                "domainSeparator":2,
                "workflowRunId":3,
                "bindingHash":4
            },
            "binding":{
                "workflowRunId":"run-123",
                "policyVersion":"policy-v1",
                "receiptHash":"receipt-999",
                "domainSeparator":"zkclear:v1"
            }
        }),
        idempotency_key: "idem-1".to_string(),
        request_hash: "hash".to_string(),
        created_at: 1,
        updated_at: 1,
        status: JobStatus::Proving,
        last_error_code: None,
        last_error_message: None,
        prover_artifacts: None,
        onchain_publish: None,
        transitions: vec![JobStatusTransition {
            from_status: None,
            to_status: JobStatus::Queued,
            transitioned_at: 1,
            error_code: None,
        }],
    }
}

fn expected_fields()
-> proof_job_coordinator::service::binding_codec_service::SettlementBindingFields {
    settlement_binding_fields("run-123", "policy-v1", "receipt-999", "zkclear:v1")
}

#[test]
fn binding_passes_when_signals_match() {
    let state = test_state();
    let fields = expected_fields();
    let artifacts = ProverArtifactsRecord {
        circuit: "settlement_valid".to_string(),
        fixture_path: "fixtures/settlement_valid.fixture.json".to_string(),
        proof_path: "artifacts/settlement_valid/settlement_valid.proof.json".to_string(),
        public_path: "artifacts/settlement_valid/settlement_valid.public.json".to_string(),
        proof_json: json!({"pi_a":[]}),
        public_json: json!([
            fields.policy_version,
            fields.receipt_hash,
            fields.domain_separator,
            fields.workflow_run_id,
            fields.binding_hash,
            "1000"
        ]),
        proof_hash: "proofhash".to_string(),
        receipt_hash: "receipt-999".to_string(),
        prove_time_seconds: 1,
    };
    let res = validate_public_signal_binding(&state, &job(), &artifacts);
    assert!(res.is_ok());
}

#[test]
fn binding_fails_on_policy_mismatch() {
    let state = test_state();
    let fields = expected_fields();
    let artifacts = ProverArtifactsRecord {
        circuit: "settlement_valid".to_string(),
        fixture_path: "fixtures/settlement_valid.fixture.json".to_string(),
        proof_path: "artifacts/settlement_valid/settlement_valid.proof.json".to_string(),
        public_path: "artifacts/settlement_valid/settlement_valid.public.json".to_string(),
        proof_json: json!({"pi_a":[]}),
        public_json: json!([
            "42",
            fields.receipt_hash,
            fields.domain_separator,
            fields.workflow_run_id,
            fields.binding_hash,
            "1000"
        ]),
        proof_hash: "proofhash".to_string(),
        receipt_hash: "receipt-999".to_string(),
        prove_time_seconds: 1,
    };
    let err = validate_public_signal_binding(&state, &job(), &artifacts).expect_err("should fail");
    assert!(err.contains("NON_RETRYABLE:BINDING_MISMATCH"));
    assert!(err.contains("policyVersion"));
}
