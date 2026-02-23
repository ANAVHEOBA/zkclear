use std::str::FromStr;
use std::sync::Arc;

use ethers::contract::abigen;
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Address, Bytes, H256, U256, U64};
use sha2::{Digest, Sha256};

use crate::errors::PublishError;
use crate::models::{PublishReceiptRequest, PublishReceiptResponse, StoredReceiptRecord};

abigen!(
    SettlementRegistry,
    r#"[
        {
            "inputs": [
                {
                    "components": [
                        {"internalType":"bytes32","name":"workflowRunId","type":"bytes32"},
                        {"internalType":"bytes32","name":"proofHash","type":"bytes32"},
                        {"internalType":"uint64","name":"policyVersion","type":"uint64"},
                        {"internalType":"uint8","name":"status","type":"uint8"},
                        {"internalType":"bytes32","name":"receiptHash","type":"bytes32"},
                        {"internalType":"bytes","name":"proof","type":"bytes"},
                        {"internalType":"uint256[]","name":"publicSignals","type":"uint256[]"}
                    ],
                    "internalType":"struct ISettlementRegistry.PublishParams",
                    "name":"params",
                    "type":"tuple"
                }
            ],
            "name":"publishReceipt",
            "outputs": [],
            "stateMutability":"nonpayable",
            "type":"function"
        }
    ]"#
);

pub struct ChainConfig {
    pub rpc_url: String,
    pub private_key: String,
    pub chain_id: u64,
}

pub async fn process_publish_receipt_onchain(
    req: PublishReceiptRequest,
    cfg: ChainConfig,
) -> Result<PublishReceiptResponse, PublishError> {
    let provider = Provider::<Http>::try_from(cfg.rpc_url.as_str())
        .map_err(|e| PublishError::Onchain(format!("provider init failed: {e}")))?;

    let wallet: LocalWallet = cfg
        .private_key
        .parse::<LocalWallet>()
        .map_err(|e| PublishError::Onchain(format!("invalid private key: {e}")))?
        .with_chain_id(cfg.chain_id);

    let signer_addr = wallet.address();
    let middleware = Arc::new(SignerMiddleware::new(provider, wallet));

    let registry_addr = Address::from_str(&req.settlement_registry)
        .map_err(|e| PublishError::Onchain(format!("invalid settlement_registry address: {e}")))?;

    if !req.publisher_address.is_empty() {
        let publisher_addr = Address::from_str(&req.publisher_address)
            .map_err(|e| PublishError::Onchain(format!("invalid publisher_address: {e}")))?;
        if publisher_addr != signer_addr {
            return Err(PublishError::Onchain(
                "publisher_address does not match private key signer".to_string(),
            ));
        }
    }

    let contract = SettlementRegistry::new(registry_addr, middleware.clone());

    let workflow_run_id = to_h256(&req.workflow_run_id);
    let proof_hash = to_h256(&req.proof_hash);
    let receipt_hash = to_h256(&req.receipt_hash);
    let status = to_status_u8(&req)?;
    let proof = decode_hex_bytes(&req.proof_hex)?;
    let public_signals = parse_public_signals(&req.public_signals)?;

    let params = PublishParams {
        workflow_run_id: workflow_run_id.to_fixed_bytes(),
        proof_hash: proof_hash.to_fixed_bytes(),
        policy_version: req.policy_version,
        status,
        receipt_hash: receipt_hash.to_fixed_bytes(),
        proof: Bytes::from(proof),
        public_signals,
    };

    let call = contract.publish_receipt(params);
    let pending = call
        .send()
        .await
        .map_err(|e| PublishError::Onchain(format!("publishReceipt call failed: {e}")))?;

    let tx_hash = pending.tx_hash();
    let receipt = pending
        .await
        .map_err(|e| PublishError::Onchain(format!("tx confirmation failed: {e}")))?
        .ok_or_else(|| PublishError::Onchain("missing transaction receipt".to_string()))?;

    if receipt.status != Some(U64::from(1u64)) {
        return Err(PublishError::Onchain(format!(
            "publishReceipt reverted onchain: tx={:#x}",
            tx_hash
        )));
    }

    let onchain_receipt_event_id = if receipt.logs.is_empty() {
        format!("{:#x}:0", tx_hash)
    } else {
        format!("{:#x}:{}", tx_hash, receipt.logs[0].log_index.unwrap_or_default())
    };

    let stored_receipt_record = StoredReceiptRecord {
        workflow_run_id: req.workflow_run_id,
        proof_hash: req.proof_hash,
        policy_version: req.policy_version,
        status: req.status,
        receipt_hash: req.receipt_hash,
    };

    Ok(PublishReceiptResponse {
        settlement_registry: req.settlement_registry,
        tx_hash: format!("{tx_hash:#x}"),
        onchain_receipt_event_id,
        stored_receipt_record,
    })
}

fn to_status_u8(req: &PublishReceiptRequest) -> Result<u8, PublishError> {
    let v = match req.status {
        crate::models::SettlementStatus::Accepted => 1u8,
        crate::models::SettlementStatus::Rejected => 2u8,
        crate::models::SettlementStatus::Settled => 3u8,
        crate::models::SettlementStatus::Failed => 4u8,
    };
    Ok(v)
}

fn decode_hex_bytes(input: &str) -> Result<Vec<u8>, PublishError> {
    let stripped = input.strip_prefix("0x").unwrap_or(input);
    hex::decode(stripped).map_err(|e| PublishError::Onchain(format!("invalid proof_hex: {e}")))
}

fn parse_public_signals(signals: &[String]) -> Result<Vec<U256>, PublishError> {
    let mut out = Vec::with_capacity(signals.len());
    for s in signals {
        let value = if let Some(hexv) = s.strip_prefix("0x") {
            U256::from_str_radix(hexv, 16)
                .map_err(|e| PublishError::Onchain(format!("invalid hex public signal `{s}`: {e}")))?
        } else {
            U256::from_dec_str(s)
                .map_err(|e| PublishError::Onchain(format!("invalid decimal public signal `{s}`: {e}")))?
        };
        out.push(value);
    }
    Ok(out)
}

fn to_h256(input: &str) -> H256 {
    if let Ok(parsed) = H256::from_str(input) {
        return parsed;
    }

    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    H256::from_slice(&hasher.finalize())
}
