use crate::service::hash_service::sha256_hex;

pub struct SettlementBindingFields {
    pub workflow_run_id: String,
    pub policy_version: String,
    pub receipt_hash: String,
    pub domain_separator: String,
    pub binding_hash: String,
}

pub fn settlement_binding_fields(
    workflow_run_id: &str,
    policy_version: &str,
    receipt_hash: &str,
    domain_separator: &str,
) -> SettlementBindingFields {
    let workflow = to_field_decimal(workflow_run_id);
    let policy = to_field_decimal(policy_version);
    let receipt = to_field_decimal(receipt_hash);
    let domain = to_field_decimal(domain_separator);

    let binding_hash = workflow
        .saturating_mul(23)
        .saturating_add(policy.saturating_mul(131))
        .saturating_add(receipt.saturating_mul(17))
        .saturating_add(domain.saturating_mul(19));

    SettlementBindingFields {
        workflow_run_id: workflow.to_string(),
        policy_version: policy.to_string(),
        receipt_hash: receipt.to_string(),
        domain_separator: domain.to_string(),
        binding_hash: binding_hash.to_string(),
    }
}

pub fn to_field_decimal(input: &str) -> u128 {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return 0;
    }
    if trimmed.bytes().all(|b| b.is_ascii_digit()) {
        if let Ok(v) = trimmed.parse::<u128>() {
            return v;
        }
    }

    // Deterministic fallback for non-numeric identifiers.
    // Match onchain projection: uint64(uint256(bytes32(hash))) -> low 8 bytes.
    let hex = sha256_hex(trimmed);
    let suffix = &hex[hex.len() - 16..];
    u64::from_str_radix(suffix, 16).unwrap_or(0) as u128
}
