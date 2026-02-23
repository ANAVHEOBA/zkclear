use crate::app::AppState;
use crate::module::proof_job::model::{OnchainPublishRecord, ProofJobRecord};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::H256;
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
    let proof_hex = value_string(&job.receipt_context, &["proofHex", "proof_hex"])
        .unwrap_or_else(|| "0x00".to_string());

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
