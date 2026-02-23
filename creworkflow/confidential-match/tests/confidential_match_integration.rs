use confidential_match::errors::MatchError;
use confidential_match::handler::process_confidential_match;
use confidential_match::models::{
    ConfidentialMatchRequest, ExternalSignals, MatchDecision, NormalizedIntent, PolicyContext, Side,
};

fn request_template() -> ConfidentialMatchRequest {
    ConfidentialMatchRequest {
        workflow_run_id: "run-123".to_string(),
        policy: PolicyContext {
            policy_version: 1,
            expected_policy_version: 1,
            max_risk_score: 70,
            max_notional: 500_000.0,
        },
        intents: vec![
            NormalizedIntent {
                intent_id: "intent-buy".to_string(),
                signer: "0xabc".to_string(),
                asset_pair: "ETH/USDC".to_string(),
                side: Side::Buy,
                size: 5.0,
                limit_price: 3020.0,
            },
            NormalizedIntent {
                intent_id: "intent-sell".to_string(),
                signer: "0xdef".to_string(),
                asset_pair: "ETH/USDC".to_string(),
                side: Side::Sell,
                size: 4.0,
                limit_price: 3000.0,
            },
        ],
        external_signals: ExternalSignals {
            api_available: true,
            compliance_passed: true,
            risk_score: 42,
            attestation_payload: "provider=risk_api;result=ok".to_string(),
        },
    }
}

#[test]
fn confidential_match_success() {
    let req = request_template();
    let out = process_confidential_match(req).expect("should match");

    assert_eq!(out.match_decision, MatchDecision::Accept);
    assert!(out.policy_check_result.passed);
    assert_eq!(out.policy_check_result.policy_version, 1);
    assert_eq!(out.private_settlement_params.execution_size, 4.0);
    assert!(!out.compliance_attestation_hash.is_empty());
}

#[test]
fn confidential_match_fails_when_api_unavailable() {
    let mut req = request_template();
    req.external_signals.api_available = false;

    let err = process_confidential_match(req).expect_err("must fail");
    assert!(matches!(err, MatchError::ApiUnavailable));
}

#[test]
fn confidential_match_fails_when_policy_mismatch() {
    let mut req = request_template();
    req.policy.policy_version = 2;
    req.policy.expected_policy_version = 1;

    let err = process_confidential_match(req).expect_err("must fail");
    assert!(matches!(err, MatchError::PolicyMismatch { .. }));
}

#[test]
fn confidential_match_fails_on_risk_threshold() {
    let mut req = request_template();
    req.external_signals.risk_score = 99;
    req.policy.max_risk_score = 70;

    let err = process_confidential_match(req).expect_err("must fail");
    assert!(matches!(err, MatchError::RiskThresholdFail { .. }));
}

#[test]
fn confidential_match_fails_on_no_match() {
    let mut req = request_template();
    req.intents[0].limit_price = 2900.0;
    req.intents[1].limit_price = 3000.0;

    let err = process_confidential_match(req).expect_err("must fail");
    assert!(matches!(err, MatchError::NoMatch));
}
