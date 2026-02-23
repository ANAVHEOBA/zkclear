use sha2::{Digest, Sha256};

use crate::errors::PublishError;
use crate::models::{PublishReceiptRequest, PublishReceiptResponse, StoredReceiptRecord};

pub fn process_publish_receipt(req: PublishReceiptRequest) -> Result<PublishReceiptResponse, PublishError> {
    validate_shape(&req)?;
    enforce_contract_checks(&req)?;

    let tx_hash = hash_hex(&[
        b"tx",
        req.settlement_registry.as_bytes(),
        req.publisher_address.as_bytes(),
        req.workflow_run_id.as_bytes(),
        req.proof_hash.as_bytes(),
        req.receipt_hash.as_bytes(),
        &req.policy_version.to_be_bytes(),
    ]);

    let onchain_receipt_event_id = hash_hex(&[
        b"event",
        req.workflow_run_id.as_bytes(),
        req.receipt_hash.as_bytes(),
        req.status_as_str().as_bytes(),
    ]);

    let stored_receipt_record = StoredReceiptRecord {
        workflow_run_id: req.workflow_run_id,
        proof_hash: req.proof_hash,
        policy_version: req.policy_version,
        status: req.status,
        receipt_hash: req.receipt_hash,
    };

    Ok(PublishReceiptResponse {
        settlement_registry: req.settlement_registry,
        tx_hash,
        onchain_receipt_event_id,
        stored_receipt_record,
    })
}

fn validate_shape(req: &PublishReceiptRequest) -> Result<(), PublishError> {
    if req.settlement_registry.trim().is_empty() {
        return Err(PublishError::InvalidRequest(
            "settlement_registry cannot be empty".to_string(),
        ));
    }
    if req.publisher_address.trim().is_empty() {
        return Err(PublishError::InvalidRequest(
            "publisher_address cannot be empty".to_string(),
        ));
    }
    if req.workflow_run_id.trim().is_empty() {
        return Err(PublishError::InvalidRequest(
            "workflow_run_id cannot be empty".to_string(),
        ));
    }
    if req.proof_hash.trim().is_empty() || req.receipt_hash.trim().is_empty() {
        return Err(PublishError::InvalidRequest(
            "proof_hash and receipt_hash are required".to_string(),
        ));
    }
    if req.policy_version == 0 {
        return Err(PublishError::InvalidRequest(
            "policy_version must be non-zero".to_string(),
        ));
    }
    if req.proof_hex.trim().is_empty() {
        return Err(PublishError::InvalidRequest(
            "proof_hex cannot be empty".to_string(),
        ));
    }
    if req.public_signals.is_empty() {
        return Err(PublishError::InvalidRequest(
            "public_signals cannot be empty".to_string(),
        ));
    }
    if req.public_signals.len() < 6 {
        return Err(PublishError::InvalidRequest(
            "public_signals must include binding/run/receipt/policy/domain/notional".to_string(),
        ));
    }
    Ok(())
}

fn enforce_contract_checks(req: &PublishReceiptRequest) -> Result<(), PublishError> {
    if !req.chain_validation.authorized_publisher {
        return Err(PublishError::UnauthorizedCaller);
    }
    if !req.chain_validation.policy_active {
        return Err(PublishError::StalePolicy);
    }
    if !req.chain_validation.proof_valid {
        return Err(PublishError::InvalidProof);
    }
    if !req.chain_validation.signal_binding_valid {
        return Err(PublishError::InvalidSignalBinding);
    }
    if req.chain_validation.duplicate_workflow_run {
        return Err(PublishError::DuplicateWorkflowRun);
    }
    if req.chain_validation.duplicate_receipt_hash {
        return Err(PublishError::DuplicateReceiptHash);
    }
    Ok(())
}

fn hash_hex(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    format!("0x{}", hex::encode(hasher.finalize()))
}

trait StatusRender {
    fn status_as_str(&self) -> &'static str;
}

impl StatusRender for PublishReceiptRequest {
    fn status_as_str(&self) -> &'static str {
        match self.status {
            crate::models::SettlementStatus::Accepted => "accepted",
            crate::models::SettlementStatus::Rejected => "rejected",
            crate::models::SettlementStatus::Settled => "settled",
            crate::models::SettlementStatus::Failed => "failed",
        }
    }
}
