use publish_receipt::errors::PublishError;
use publish_receipt::handler::process_publish_receipt;
use publish_receipt::models::{ChainValidationState, PublishReceiptRequest, SettlementStatus};

fn request_template() -> PublishReceiptRequest {
    PublishReceiptRequest {
        settlement_registry: "0x3e3a14f46d13e156daa99bf234224a57b1c79da5".to_string(),
        publisher_address: "0x6D21167d874C842386e8c484519B5ddBBaB87b43".to_string(),
        workflow_run_id: "run-1001".to_string(),
        proof_hash: "0xproof-hash-1".to_string(),
        policy_version: 1,
        status: SettlementStatus::Settled,
        receipt_hash: "0xreceipt-hash-1".to_string(),
        proof_hex: "0xc0ffee".to_string(),
        public_signals: vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
            "5".to_string(),
            "6".to_string(),
        ],
        chain_validation: ChainValidationState {
            authorized_publisher: true,
            policy_active: true,
            proof_valid: true,
            signal_binding_valid: true,
            duplicate_workflow_run: false,
            duplicate_receipt_hash: false,
        },
    }
}

#[test]
fn publish_receipt_success() {
    let req = request_template();
    let out = process_publish_receipt(req).expect("publish should succeed");

    assert_eq!(
        out.settlement_registry,
        "0x3e3a14f46d13e156daa99bf234224a57b1c79da5"
    );
    assert!(out.tx_hash.starts_with("0x"));
    assert!(out.onchain_receipt_event_id.starts_with("0x"));
    assert_eq!(out.stored_receipt_record.policy_version, 1);
    assert_eq!(out.stored_receipt_record.status, SettlementStatus::Settled);
}

#[test]
fn publish_receipt_fails_on_invalid_proof() {
    let mut req = request_template();
    req.chain_validation.proof_valid = false;

    let err = process_publish_receipt(req).expect_err("must fail");
    assert!(matches!(err, PublishError::InvalidProof));
}

#[test]
fn publish_receipt_fails_on_stale_policy() {
    let mut req = request_template();
    req.chain_validation.policy_active = false;

    let err = process_publish_receipt(req).expect_err("must fail");
    assert!(matches!(err, PublishError::StalePolicy));
}

#[test]
fn publish_receipt_fails_on_duplicate_run_hash() {
    let mut req = request_template();
    req.chain_validation.duplicate_workflow_run = true;

    let err = process_publish_receipt(req).expect_err("must fail");
    assert!(matches!(err, PublishError::DuplicateWorkflowRun));

    let mut req2 = request_template();
    req2.chain_validation.duplicate_receipt_hash = true;

    let err2 = process_publish_receipt(req2).expect_err("must fail");
    assert!(matches!(err2, PublishError::DuplicateReceiptHash));
}

#[test]
fn publish_receipt_fails_on_unauthorized_caller() {
    let mut req = request_template();
    req.chain_validation.authorized_publisher = false;

    let err = process_publish_receipt(req).expect_err("must fail");
    assert!(matches!(err, PublishError::UnauthorizedCaller));
}
