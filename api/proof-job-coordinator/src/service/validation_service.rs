use crate::module::proof_job::error::AppError;
use crate::module::proof_job::schema::{ProofType, SubmitProofJobRequest};

pub fn validate_submit_request(req: &SubmitProofJobRequest) -> Result<(), AppError> {
    if req.workflow_run_id.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_WORKFLOW_RUN_ID",
            "workflow_run_id is required",
        ));
    }
    if req.policy_version.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_POLICY_VERSION",
            "policy_version is required",
        ));
    }
    if req.idempotency_key.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_IDEMPOTENCY_KEY",
            "idempotency_key is required",
        ));
    }
    if !req
        .idempotency_key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::bad_request(
            "INVALID_IDEMPOTENCY_KEY",
            "idempotency_key contains invalid characters",
        ));
    }

    if !req.receipt_context.is_object() {
        return Err(AppError::bad_request(
            "INVALID_RECEIPT_CONTEXT",
            "receipt_context must be a JSON object",
        ));
    }

    match req.proof_type {
        ProofType::Settlement | ProofType::Compliance | ProofType::Rebate => {}
    }

    Ok(())
}
