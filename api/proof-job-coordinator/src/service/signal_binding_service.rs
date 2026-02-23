use crate::app::AppState;
use crate::module::proof_job::model::{ProofJobRecord, ProverArtifactsRecord};
use serde_json::Value;

pub fn validate_public_signal_binding(
    state: &AppState,
    job: &ProofJobRecord,
    artifacts: &ProverArtifactsRecord,
) -> Result<(), String> {
    let signals = artifacts.public_json.as_array().ok_or_else(|| {
        non_retryable(
            "BINDING_INVALID_PUBLIC_SIGNALS",
            "public.json must be an array",
        )
    })?;

    let idx_workflow = binding_index(&job.receipt_context, "workflowRunId", 0)?;
    let idx_policy = binding_index(&job.receipt_context, "policyVersion", 1)?;
    let idx_receipt = binding_index(&job.receipt_context, "receiptHash", 2)?;
    let idx_domain = binding_index(&job.receipt_context, "domainSeparator", 3)?;

    let expected_workflow = expected_binding_value(&job.receipt_context, "workflowRunId")
        .unwrap_or_else(|| job.workflow_run_id.clone());
    let expected_policy = expected_binding_value(&job.receipt_context, "policyVersion")
        .unwrap_or_else(|| job.policy_version.clone());
    let expected_receipt = expected_binding_value(&job.receipt_context, "receiptHash")
        .unwrap_or_else(|| artifacts.receipt_hash.clone());
    let expected_domain = expected_binding_value(&job.receipt_context, "domainSeparator")
        .unwrap_or_else(|| state.config.signal_domain_separator.clone());

    compare_signal(signals, idx_workflow, "workflowRunId", &expected_workflow)?;
    compare_signal(signals, idx_policy, "policyVersion", &expected_policy)?;
    compare_signal(signals, idx_receipt, "receiptHash", &expected_receipt)?;
    compare_signal(signals, idx_domain, "domainSeparator", &expected_domain)?;

    Ok(())
}

fn binding_index(context: &Value, key: &str, default_idx: usize) -> Result<usize, String> {
    let idx_opt = context
        .get("publicSignalIndex")
        .or_else(|| context.get("public_signal_index"))
        .and_then(|v| v.get(key))
        .and_then(Value::as_u64);

    let idx = idx_opt.unwrap_or(default_idx as u64);
    usize::try_from(idx)
        .map_err(|_| non_retryable("BINDING_INVALID_INDEX", format!("invalid index for {key}")))
}

fn expected_binding_value(context: &Value, key: &str) -> Option<String> {
    context
        .get("binding")
        .or_else(|| context.get("expectedBindings"))
        .or_else(|| context.get("expected_bindings"))
        .and_then(|v| v.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn compare_signal(
    signals: &[Value],
    idx: usize,
    label: &str,
    expected: &str,
) -> Result<(), String> {
    let actual = signals
        .get(idx)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            non_retryable(
                "BINDING_SIGNAL_MISSING",
                format!("public signal at index {idx} for {label} not found"),
            )
        })?
        .trim();

    if actual != expected.trim() {
        return Err(non_retryable(
            "BINDING_MISMATCH",
            format!(
                "{label} mismatch: expected={} actual={}",
                expected.trim(),
                actual
            ),
        ));
    }
    Ok(())
}

fn non_retryable(code: &str, message: impl Into<String>) -> String {
    format!("NON_RETRYABLE:{code}:{}", message.into())
}
