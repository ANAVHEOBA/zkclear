use sha2::{Digest, Sha256};

use crate::errors::MatchError;
use crate::models::{
    ConfidentialMatchRequest, ConfidentialMatchResponse, MatchDecision, NormalizedIntent, PolicyCheckResult,
    SettlementParams, Side,
};

pub fn process_confidential_match(req: ConfidentialMatchRequest) -> Result<ConfidentialMatchResponse, MatchError> {
    validate_request_shape(&req)?;
    ensure_api_available(&req)?;
    ensure_policy_version(&req)?;
    ensure_risk_and_compliance(&req)?;

    let (buy, sell) = select_pair(&req.intents)?;
    let settlement = build_settlement(buy, sell, req.policy.max_notional)?;
    let policy_check_result = PolicyCheckResult {
        passed: true,
        policy_version: req.policy.policy_version,
        reason: "all policy checks passed".to_string(),
    };

    Ok(ConfidentialMatchResponse {
        workflow_run_id: req.workflow_run_id.clone(),
        match_decision: MatchDecision::Accept,
        private_settlement_params: settlement,
        policy_check_result,
        compliance_attestation_hash: attestation_hash(
            &req.workflow_run_id,
            req.policy.policy_version,
            &req.external_signals.attestation_payload,
        ),
    })
}

fn validate_request_shape(req: &ConfidentialMatchRequest) -> Result<(), MatchError> {
    if req.workflow_run_id.trim().is_empty() {
        return Err(MatchError::InvalidRequest(
            "workflow_run_id cannot be empty".to_string(),
        ));
    }
    if req.intents.len() != 2 {
        return Err(MatchError::InvalidRequest(
            "exactly two intents are required".to_string(),
        ));
    }
    Ok(())
}

fn ensure_api_available(req: &ConfidentialMatchRequest) -> Result<(), MatchError> {
    if !req.external_signals.api_available {
        return Err(MatchError::ApiUnavailable);
    }
    Ok(())
}

fn ensure_policy_version(req: &ConfidentialMatchRequest) -> Result<(), MatchError> {
    if req.policy.policy_version != req.policy.expected_policy_version {
        return Err(MatchError::PolicyMismatch {
            expected: req.policy.expected_policy_version,
            got: req.policy.policy_version,
        });
    }
    Ok(())
}

fn ensure_risk_and_compliance(req: &ConfidentialMatchRequest) -> Result<(), MatchError> {
    if req.external_signals.risk_score > req.policy.max_risk_score {
        return Err(MatchError::RiskThresholdFail {
            risk_score: req.external_signals.risk_score,
            max: req.policy.max_risk_score,
        });
    }
    if !req.external_signals.compliance_passed {
        return Err(MatchError::ComplianceFail);
    }
    Ok(())
}

fn select_pair(intents: &[NormalizedIntent]) -> Result<(&NormalizedIntent, &NormalizedIntent), MatchError> {
    let first = &intents[0];
    let second = &intents[1];

    let (buy, sell) = match (&first.side, &second.side) {
        (Side::Buy, Side::Sell) => (first, second),
        (Side::Sell, Side::Buy) => (second, first),
        _ => return Err(MatchError::NoMatch),
    };

    if buy.asset_pair != sell.asset_pair {
        return Err(MatchError::NoMatch);
    }
    if buy.limit_price < sell.limit_price {
        return Err(MatchError::NoMatch);
    }
    if buy.size <= 0.0 || sell.size <= 0.0 {
        return Err(MatchError::NoMatch);
    }

    Ok((buy, sell))
}

fn build_settlement(
    buy: &NormalizedIntent,
    sell: &NormalizedIntent,
    max_notional: f64,
) -> Result<SettlementParams, MatchError> {
    let execution_size = buy.size.min(sell.size);
    let execution_price = (buy.limit_price + sell.limit_price) / 2.0;
    let notional = execution_size * execution_price;

    if notional > max_notional {
        return Err(MatchError::NoMatch);
    }

    Ok(SettlementParams {
        asset_pair: buy.asset_pair.clone(),
        buy_intent_id: buy.intent_id.clone(),
        sell_intent_id: sell.intent_id.clone(),
        execution_size,
        execution_price,
        notional,
    })
}

fn attestation_hash(workflow_run_id: &str, policy_version: u64, payload: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workflow_run_id.as_bytes());
    hasher.update(policy_version.to_be_bytes());
    hasher.update(payload.as_bytes());
    format!("0x{}", hex::encode(hasher.finalize()))
}
