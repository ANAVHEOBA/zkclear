use settle_private::errors::SettleError;
use settle_private::handler::process_settle_private;
use settle_private::models::{
    ExecutionControl, ProofBundle, SettlePrivateRequest, SettlementInstruction, SettlementStatus,
};

fn request_template() -> SettlePrivateRequest {
    SettlePrivateRequest {
        workflow_run_id: "run-900".to_string(),
        proof_bundle: ProofBundle {
            proof_hash: "0xproofhash".to_string(),
            receipt_hash: "0xreceipthash".to_string(),
            approved: true,
        },
        settlement_instruction: SettlementInstruction {
            asset: "USDC".to_string(),
            amount: 10000.0,
            from_account: "vault_a".to_string(),
            to_account: "vault_b".to_string(),
            transfer_simulation_ok: true,
            counterparty_conflict: false,
        },
        execution: ExecutionControl {
            max_retries: 2,
            timeout_ms: 3000,
            estimated_execution_ms: 500,
            retryable_error_sequence: vec![false],
        },
    }
}

#[test]
fn settle_private_success() {
    let req = request_template();
    let out = process_settle_private(req).expect("settlement should succeed");

    assert_eq!(out.settlement_status, SettlementStatus::Settled);
    assert_eq!(out.private_execution_reference_ids.len(), 2);
    assert_eq!(out.attempts_used, 1);
}

#[test]
fn settle_private_fails_on_transfer_failure() {
    let mut req = request_template();
    req.settlement_instruction.transfer_simulation_ok = false;

    let err = process_settle_private(req).expect_err("must fail");
    assert!(matches!(err, SettleError::TransferFailure));
}

#[test]
fn settle_private_fails_on_counterparty_conflict() {
    let mut req = request_template();
    req.settlement_instruction.counterparty_conflict = true;

    let err = process_settle_private(req).expect_err("must fail");
    assert!(matches!(err, SettleError::CounterpartySettlementConflict));
}

#[test]
fn settle_private_fails_on_timeout_retry_exhausted() {
    let mut req = request_template();
    req.execution.max_retries = 2;
    req.execution.retryable_error_sequence = vec![true, true, true];

    let err = process_settle_private(req).expect_err("must fail");
    assert!(matches!(err, SettleError::TimeoutRetryExhausted { .. }));
}
