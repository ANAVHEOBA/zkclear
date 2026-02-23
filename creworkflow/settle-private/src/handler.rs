use sha2::{Digest, Sha256};

use crate::errors::SettleError;
use crate::models::{SettlePrivateRequest, SettlePrivateResponse, SettlementStatus};

pub fn process_settle_private(req: SettlePrivateRequest) -> Result<SettlePrivateResponse, SettleError> {
    validate_request(&req)?;

    if req.settlement_instruction.counterparty_conflict {
        return Err(SettleError::CounterpartySettlementConflict);
    }

    if !req.settlement_instruction.transfer_simulation_ok {
        return Err(SettleError::TransferFailure);
    }

    if req.execution.estimated_execution_ms > req.execution.timeout_ms {
        return Err(SettleError::TimeoutRetryExhausted {
            attempts: 1,
            max_retries: req.execution.max_retries,
        });
    }

    let max_attempts = req.execution.max_retries.saturating_add(1);
    let mut attempts_used: u32 = 0;
    let mut success = false;

    for attempt in 1..=max_attempts {
        attempts_used = attempt;
        let is_retryable_error = req
            .execution
            .retryable_error_sequence
            .get((attempt - 1) as usize)
            .copied()
            .unwrap_or(false);

        if is_retryable_error {
            continue;
        }

        success = true;
        break;
    }

    if !success {
        return Err(SettleError::TimeoutRetryExhausted {
            attempts: attempts_used,
            max_retries: req.execution.max_retries,
        });
    }

    let ref_a = execution_ref(&req.workflow_run_id, "ledger-leg-a", attempts_used);
    let ref_b = execution_ref(&req.workflow_run_id, "ledger-leg-b", attempts_used);

    Ok(SettlePrivateResponse {
        workflow_run_id: req.workflow_run_id,
        settlement_status: SettlementStatus::Settled,
        private_execution_reference_ids: vec![ref_a, ref_b],
        attempts_used,
    })
}

fn validate_request(req: &SettlePrivateRequest) -> Result<(), SettleError> {
    if req.workflow_run_id.trim().is_empty() {
        return Err(SettleError::InvalidRequest(
            "workflow_run_id cannot be empty".to_string(),
        ));
    }
    if !req.proof_bundle.approved {
        return Err(SettleError::InvalidRequest(
            "proof bundle not approved".to_string(),
        ));
    }
    if req.proof_bundle.proof_hash.trim().is_empty() || req.proof_bundle.receipt_hash.trim().is_empty() {
        return Err(SettleError::InvalidRequest(
            "proof_hash and receipt_hash are required".to_string(),
        ));
    }
    if req.settlement_instruction.amount <= 0.0 {
        return Err(SettleError::InvalidRequest(
            "settlement amount must be positive".to_string(),
        ));
    }
    if req.settlement_instruction.from_account == req.settlement_instruction.to_account {
        return Err(SettleError::InvalidRequest(
            "from_account and to_account must differ".to_string(),
        ));
    }
    Ok(())
}

fn execution_ref(workflow_run_id: &str, leg: &str, attempts: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workflow_run_id.as_bytes());
    hasher.update(leg.as_bytes());
    hasher.update(attempts.to_be_bytes());
    format!("exec_{}", hex::encode(hasher.finalize()))
}
