use proof_generate::errors::ProofError;
use proof_generate::handler::process_proof_generate;
use proof_generate::models::{MatchResult, PolicyResult, ProofGenerateRequest, SettlementParams};

fn request_template() -> ProofGenerateRequest {
    ProofGenerateRequest {
        workflow_run_id: "run-777".to_string(),
        match_result: MatchResult { accepted: true },
        policy_result: PolicyResult {
            passed: true,
            policy_version: 1,
        },
        settlement_params: SettlementParams {
            asset_pair: "ETH/USDC".to_string(),
            buy_intent_id: "intent-buy".to_string(),
            sell_intent_id: "intent-sell".to_string(),
            execution_size: 4.0,
            execution_price: 3000.0,
            notional: 12000.0,
        },
        proving_timeout_ms: 3000,
        estimated_proving_time_ms: 700,
        domain_separator: "zkclear-sepolia-domain-v1".to_string(),
        witness_seed: "witness-seed-123456".to_string(),
    }
}

#[test]
fn proof_generate_success() {
    let req = request_template();
    let out = process_proof_generate(req).expect("proof generation should succeed");

    assert!(!out.proof_bytes.is_empty());
    assert_eq!(out.public_signals.len(), 6);
    assert!(out.proof_hash.starts_with("0x"));
    assert!(out.receipt_hash.starts_with("0x"));
    assert_eq!(out.policy_version, 1);
    assert!(out.domain_binding_hash.starts_with("0x"));
}

#[test]
fn proof_generate_fails_on_constraint_failure() {
    let mut req = request_template();
    req.match_result.accepted = false;

    let err = process_proof_generate(req).expect_err("must fail");
    assert!(matches!(err, ProofError::CircuitConstraintFailure(_)));
}

#[test]
fn proof_generate_fails_on_witness_generation_failure() {
    let mut req = request_template();
    req.witness_seed = "".to_string();

    let err = process_proof_generate(req).expect_err("must fail");
    assert!(matches!(err, ProofError::WitnessGenerationFailure(_)));
}

#[test]
fn proof_generate_fails_on_timeout() {
    let mut req = request_template();
    req.proving_timeout_ms = 100;
    req.estimated_proving_time_ms = 500;

    let err = process_proof_generate(req).expect_err("must fail");
    assert!(matches!(err, ProofError::ProvingTimeout { .. }));
}
