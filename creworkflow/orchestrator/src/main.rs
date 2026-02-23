use std::io::{self, Read};
use std::time::Duration;

use confidential_match::handler::process_confidential_match;
use confidential_match::models::{
    ConfidentialMatchRequest, ExternalSignals, MatchDecision, NormalizedIntent as MatchIntent, PolicyContext,
    Side as MatchSide,
};
use intent_intake::handler::process_intake;
use intent_intake::models::IntentIntakeRequest;
use proof_generate::handler::process_proof_generate;
use proof_generate::models::{MatchResult, PolicyResult, ProofGenerateRequest, SettlementParams as ProofSettlementParams};
use publish_receipt::handler::process_publish_receipt;
use publish_receipt::models::{
    ChainValidationState, PublishReceiptRequest, PublishReceiptResponse, SettlementStatus as PublishStatus,
};
use serde::{Deserialize, Serialize};
use settle_private::handler::process_settle_private;
use settle_private::models::{
    ExecutionControl, ProofBundle, SettlePrivateRequest, SettlementInstruction, SettlementStatus as PrivateStatus,
};

#[derive(Debug, Deserialize)]
struct OrchestratorRequest {
    intent_intake: IntentIntakeRequest,
    policy: OrchestratorPolicy,
    external_signals: Option<OrchestratorSignals>,
    confidential_http: Option<ConfidentialHttpConfig>,
    proving: ProvingConfig,
    settlement_execution: SettlementExecConfig,
    publish: PublishConfig,
}

#[derive(Debug, Deserialize)]
struct OrchestratorPolicy {
    policy_version: u64,
    expected_policy_version: u64,
    max_risk_score: u32,
    max_notional: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct OrchestratorSignals {
    api_available: bool,
    compliance_passed: bool,
    risk_score: u32,
    attestation_payload: String,
}

#[derive(Debug, Deserialize)]
struct ConfidentialHttpConfig {
    enabled: bool,
    endpoint: String,
    timeout_ms: Option<u64>,
    api_key_env: Option<String>,
    api_key_header: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProvingConfig {
    proving_timeout_ms: u64,
    estimated_proving_time_ms: u64,
    domain_separator: String,
    witness_seed: String,
}

#[derive(Debug, Deserialize)]
struct SettlementExecConfig {
    max_retries: u32,
    timeout_ms: u64,
    estimated_execution_ms: u64,
    retryable_error_sequence: Vec<bool>,
}

#[derive(Debug, Deserialize)]
struct PublishConfig {
    settlement_registry: String,
    publisher_address: String,
}

#[derive(Debug, Serialize)]
struct OrchestratorResponse {
    workflow_run_id: String,
    intent_commitment_hashes: Vec<String>,
    match_decision: String,
    proof_hash: String,
    receipt_hash: String,
    settlement_status: String,
    publish_result: PublishReceiptResponse,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("failed reading stdin: {e}"))?;

    let req: OrchestratorRequest =
        serde_json::from_str(&input).map_err(|e| format!("invalid json input: {e}"))?;

    let resolved_signals = resolve_external_signals(&req)?;

    let intake = process_intake(req.intent_intake).map_err(|e| format!("intent-intake failed: {e}"))?;

    let intents_for_match = intake
        .normalized_private_intents
        .iter()
        .map(|it| {
            let size = it.size.parse::<f64>().map_err(|e| format!("invalid size: {e}"))?;
            let limit_price = it
                .limit_price
                .parse::<f64>()
                .map_err(|e| format!("invalid limit_price: {e}"))?;

            Ok(MatchIntent {
                intent_id: it.intent_id.clone(),
                signer: it.signer.clone(),
                asset_pair: it.asset_pair.clone(),
                side: match it.side {
                    intent_intake::models::Side::Buy => MatchSide::Buy,
                    intent_intake::models::Side::Sell => MatchSide::Sell,
                },
                size,
                limit_price,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let match_req = ConfidentialMatchRequest {
        workflow_run_id: intake.workflow_run_id.clone(),
        policy: PolicyContext {
            policy_version: req.policy.policy_version,
            expected_policy_version: req.policy.expected_policy_version,
            max_risk_score: req.policy.max_risk_score,
            max_notional: req.policy.max_notional,
        },
        intents: intents_for_match,
        external_signals: ExternalSignals {
            api_available: resolved_signals.api_available,
            compliance_passed: resolved_signals.compliance_passed,
            risk_score: resolved_signals.risk_score,
            attestation_payload: resolved_signals.attestation_payload,
        },
    };

    let matched =
        process_confidential_match(match_req).map_err(|e| format!("confidential-match failed: {e}"))?;
    if matched.match_decision != MatchDecision::Accept {
        return Err("confidential-match rejected".to_string());
    }

    let proof_req = ProofGenerateRequest {
        workflow_run_id: intake.workflow_run_id.clone(),
        match_result: MatchResult { accepted: true },
        policy_result: PolicyResult {
            passed: matched.policy_check_result.passed,
            policy_version: matched.policy_check_result.policy_version,
        },
        settlement_params: ProofSettlementParams {
            asset_pair: matched.private_settlement_params.asset_pair.clone(),
            buy_intent_id: matched.private_settlement_params.buy_intent_id.clone(),
            sell_intent_id: matched.private_settlement_params.sell_intent_id.clone(),
            execution_size: matched.private_settlement_params.execution_size,
            execution_price: matched.private_settlement_params.execution_price,
            notional: matched.private_settlement_params.notional,
        },
        proving_timeout_ms: req.proving.proving_timeout_ms,
        estimated_proving_time_ms: req.proving.estimated_proving_time_ms,
        domain_separator: req.proving.domain_separator,
        witness_seed: req.proving.witness_seed,
    };

    let proved = process_proof_generate(proof_req).map_err(|e| format!("proof-generate failed: {e}"))?;

    let settle_req = SettlePrivateRequest {
        workflow_run_id: intake.workflow_run_id.clone(),
        proof_bundle: ProofBundle {
            proof_hash: proved.proof_hash.clone(),
            receipt_hash: proved.receipt_hash.clone(),
            approved: true,
        },
        settlement_instruction: SettlementInstruction {
            asset: matched.private_settlement_params.asset_pair.clone(),
            amount: matched.private_settlement_params.notional,
            from_account: matched.private_settlement_params.buy_intent_id.clone(),
            to_account: matched.private_settlement_params.sell_intent_id.clone(),
            transfer_simulation_ok: true,
            counterparty_conflict: false,
        },
        execution: ExecutionControl {
            max_retries: req.settlement_execution.max_retries,
            timeout_ms: req.settlement_execution.timeout_ms,
            estimated_execution_ms: req.settlement_execution.estimated_execution_ms,
            retryable_error_sequence: req.settlement_execution.retryable_error_sequence,
        },
    };

    let settled = process_settle_private(settle_req).map_err(|e| format!("settle-private failed: {e}"))?;
    if settled.settlement_status != PrivateStatus::Settled {
        return Err("settle-private did not settle".to_string());
    }

    let publish_req = PublishReceiptRequest {
        settlement_registry: req.publish.settlement_registry,
        publisher_address: req.publish.publisher_address,
        workflow_run_id: intake.workflow_run_id.clone(),
        proof_hash: proved.proof_hash.clone(),
        policy_version: proved.policy_version,
        status: PublishStatus::Settled,
        receipt_hash: proved.receipt_hash.clone(),
        proof_hex: format!("0x{}", hex::encode(&proved.proof_bytes)),
        public_signals: proved.public_signals.clone(),
        chain_validation: ChainValidationState {
            authorized_publisher: true,
            policy_active: true,
            proof_valid: true,
            signal_binding_valid: true,
            duplicate_workflow_run: false,
            duplicate_receipt_hash: false,
        },
    };

    let publish_result =
        process_publish_receipt(publish_req).map_err(|e| format!("publish-receipt failed: {e}"))?;

    let out = OrchestratorResponse {
        workflow_run_id: intake.workflow_run_id,
        intent_commitment_hashes: intake.intent_commitment_hashes,
        match_decision: "accept".to_string(),
        proof_hash: proved.proof_hash,
        receipt_hash: proved.receipt_hash,
        settlement_status: "settled".to_string(),
        publish_result,
    };

    let output = serde_json::to_string_pretty(&out).map_err(|e| format!("serialization failed: {e}"))?;
    println!("{output}");
    Ok(())
}

fn resolve_external_signals(req: &OrchestratorRequest) -> Result<OrchestratorSignals, String> {
    if let Some(cfg) = &req.confidential_http {
        if cfg.enabled {
            return fetch_signals_via_confidential_http(cfg);
        }
    }

    req.external_signals
        .clone()
        .ok_or_else(|| "external_signals missing while confidential_http disabled".to_string())
}

fn fetch_signals_via_confidential_http(cfg: &ConfidentialHttpConfig) -> Result<OrchestratorSignals, String> {
    if cfg.endpoint.trim().is_empty() {
        return Err("confidential_http.endpoint cannot be empty".to_string());
    }

    let timeout = Duration::from_millis(cfg.timeout_ms.unwrap_or(8_000));
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("failed to build http client: {e}"))?;

    let mut request = client.get(&cfg.endpoint);

    if let Some(env_name) = &cfg.api_key_env {
        let key = std::env::var(env_name).map_err(|_| {
            format!("api key env var `{env_name}` not found for confidential_http")
        })?;
        let header_name = cfg
            .api_key_header
            .clone()
            .unwrap_or_else(|| "x-api-key".to_string());
        request = request.header(header_name, key);
    }

    let response = request
        .send()
        .map_err(|e| format!("confidential http request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "confidential http non-success status: {}",
            response.status()
        ));
    }

    response
        .json::<OrchestratorSignals>()
        .map_err(|e| format!("failed to decode confidential http response json: {e}"))
}
