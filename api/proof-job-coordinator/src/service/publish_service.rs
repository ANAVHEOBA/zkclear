use crate::app::AppState;
use crate::module::proof_job::model::{OnchainPublishRecord, ProofJobRecord};
use ethers::abi::{Token, encode};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::H256;
use ethers::types::U256;
use publish_receipt::chain::{ChainConfig, process_publish_receipt_onchain};
use publish_receipt::models::{ChainValidationState, PublishReceiptRequest, SettlementStatus};
use serde_json::Value;
use std::str::FromStr;

pub async fn publish_receipt_on_sepolia(
    state: &AppState,
    job: &ProofJobRecord,
) -> Result<OnchainPublishRecord, String> {
    let settlement_registry = state
        .config
        .publish_settlement_registry
        .clone()
        .or_else(|| {
            value_string(
                &job.receipt_context,
                &["settlementRegistry", "settlement_registry"],
            )
        })
        .ok_or_else(|| "publish settlement registry is not configured".to_string())?;
    let publisher_address = state
        .config
        .publish_publisher_address
        .clone()
        .or_else(|| {
            value_string(
                &job.receipt_context,
                &["publisherAddress", "publisher_address"],
            )
        })
        .ok_or_else(|| "publish publisher address is not configured".to_string())?;

    let rpc_url = state
        .config
        .eth_sepolia_rpc_url
        .clone()
        .ok_or_else(|| "ETH_SEPOLIA_RPC_URL is required".to_string())?;
    let private_key = state
        .config
        .private_key
        .clone()
        .ok_or_else(|| "PRIVATE_KEY is required".to_string())?;

    let artifacts = job
        .prover_artifacts
        .as_ref()
        .ok_or_else(|| "missing prover artifacts for publish".to_string())?;

    let policy_version = parse_policy_version(&job.policy_version)?;
    let public_signals = to_public_signal_strings(&artifacts.public_json)?;
    let proof_hex = match value_string(&job.receipt_context, &["proofHex", "proof_hex"]) {
        Some(v) => v,
        None => encode_proof_hex_from_json(&artifacts.proof_json)
            .map_err(|e| format!("proof hex encode failed: {e}"))?,
    };

    let req = PublishReceiptRequest {
        settlement_registry: settlement_registry.clone(),
        publisher_address: publisher_address.clone(),
        workflow_run_id: job.workflow_run_id.clone(),
        proof_hash: artifacts.proof_hash.clone(),
        policy_version,
        status: SettlementStatus::Settled,
        receipt_hash: artifacts.receipt_hash.clone(),
        proof_hex,
        public_signals,
        chain_validation: ChainValidationState {
            authorized_publisher: true,
            policy_active: true,
            proof_valid: true,
            signal_binding_valid: true,
            duplicate_workflow_run: false,
            duplicate_receipt_hash: false,
        },
    };

    let resp = process_publish_receipt_onchain(
        req,
        ChainConfig {
            rpc_url: rpc_url.clone(),
            private_key,
            chain_id: state.config.eth_sepolia_chain_id,
        },
    )
    .await
    .map_err(|e| format!("publish-receipt failed: {e}"))?;

    let block_number = fetch_block_number(&rpc_url, &resp.tx_hash).await;
    Ok(OnchainPublishRecord {
        settlement_registry: resp.settlement_registry,
        tx_hash: resp.tx_hash,
        block_number,
        onchain_status: "CONFIRMED".to_string(),
        onchain_receipt_event_id: resp.onchain_receipt_event_id,
    })
}

fn parse_policy_version(v: &str) -> Result<u64, String> {
    if let Ok(parsed) = v.trim().parse::<u64>() {
        return Ok(parsed);
    }
    let digits: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err(format!("policy_version `{v}` is not numeric"));
    }
    digits
        .parse::<u64>()
        .map_err(|e| format!("invalid policy_version `{v}`: {e}"))
}

fn to_public_signal_strings(v: &Value) -> Result<Vec<String>, String> {
    let arr = v
        .as_array()
        .ok_or_else(|| "public_json must be an array".to_string())?;
    if arr.is_empty() {
        return Err("public_json cannot be empty".to_string());
    }
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        match item {
            Value::String(s) => out.push(s.trim().to_string()),
            Value::Number(n) => out.push(n.to_string()),
            _ => return Err("public_json contains unsupported signal type".to_string()),
        }
    }
    Ok(out)
}

fn value_string(v: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| v.get(*k))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn encode_proof_hex_from_json(proof_json: &Value) -> Result<String, String> {
    let p_a = proof_json
        .get("pi_a")
        .and_then(Value::as_array)
        .ok_or_else(|| "proof_json missing pi_a".to_string())?;
    let p_b = proof_json
        .get("pi_b")
        .and_then(Value::as_array)
        .ok_or_else(|| "proof_json missing pi_b".to_string())?;
    let p_c = proof_json
        .get("pi_c")
        .and_then(Value::as_array)
        .ok_or_else(|| "proof_json missing pi_c".to_string())?;
    if p_a.len() < 2 || p_b.len() < 2 || p_c.len() < 2 {
        return Err("proof_json has insufficient proof coordinates".to_string());
    }

    let p_b0 = p_b
        .first()
        .and_then(Value::as_array)
        .ok_or_else(|| "proof_json pi_b[0] missing".to_string())?;
    let p_b1 = p_b
        .get(1)
        .and_then(Value::as_array)
        .ok_or_else(|| "proof_json pi_b[1] missing".to_string())?;
    if p_b0.len() < 2 || p_b1.len() < 2 {
        return Err("proof_json pi_b coordinates incomplete".to_string());
    }

    let a0 = parse_u256_value(&p_a[0])?;
    let a1 = parse_u256_value(&p_a[1])?;
    let b00 = parse_u256_value(&p_b0[1])?;
    let b01 = parse_u256_value(&p_b0[0])?;
    let b10 = parse_u256_value(&p_b1[1])?;
    let b11 = parse_u256_value(&p_b1[0])?;
    let c0 = parse_u256_value(&p_c[0])?;
    let c1 = parse_u256_value(&p_c[1])?;

    let bytes = encode(&[
        Token::FixedArray(vec![Token::Uint(a0), Token::Uint(a1)]),
        Token::FixedArray(vec![
            Token::FixedArray(vec![Token::Uint(b00), Token::Uint(b01)]),
            Token::FixedArray(vec![Token::Uint(b10), Token::Uint(b11)]),
        ]),
        Token::FixedArray(vec![Token::Uint(c0), Token::Uint(c1)]),
    ]);

    Ok(format!("0x{}", hex::encode(bytes)))
}

fn parse_u256_value(v: &Value) -> Result<U256, String> {
    match v {
        Value::String(s) => parse_u256_string(s),
        Value::Number(n) => parse_u256_string(&n.to_string()),
        _ => Err("proof coordinate must be string or number".to_string()),
    }
}

fn parse_u256_string(s: &str) -> Result<U256, String> {
    if let Some(hexv) = s.strip_prefix("0x") {
        return U256::from_str_radix(hexv, 16)
            .map_err(|e| format!("invalid hex proof coordinate `{s}`: {e}"));
    }
    U256::from_dec_str(s).map_err(|e| format!("invalid decimal proof coordinate `{s}`: {e}"))
}

async fn fetch_block_number(rpc_url: &str, tx_hash: &str) -> Option<u64> {
    let provider = Provider::<Http>::try_from(rpc_url).ok()?;
    let parsed = H256::from_str(tx_hash).ok()?;
    let receipt = provider
        .get_transaction_receipt(parsed)
        .await
        .ok()
        .flatten()?;
    receipt.block_number.map(|n| n.as_u64())
}
