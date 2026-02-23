use num_bigint::BigUint;
use num_traits::{One, Zero};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SettlementVector {
    pub amount_in: String,
    pub amount_out: String,
    pub fee: String,
    pub execution_size: String,
    pub execution_price: String,
    pub limit_price: String,
    pub max_notional: String,
    pub notional: String,
    pub policy_version_private: String,
    pub receipt_hash_private: String,
    pub domain_separator_private: String,
    pub workflow_run_id_private: String,
    pub policy_version_public: String,
    pub receipt_hash_public: String,
    pub domain_separator_public: String,
    pub workflow_run_id_public: String,
    pub binding_hash_public: String,
    pub notional_public: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComplianceVector {
    pub compliance_pass_bit: String,
    pub risk_score: String,
    pub max_risk_score: String,
    pub policy_version_private: String,
    pub policy_version_public: String,
    pub attestation_hash_private: String,
    pub attestation_hash_public: String,
    pub sanctions_commitment_private: String,
    pub sanctions_commitment_public: String,
    pub allowlist_commitment_private: String,
    pub allowlist_commitment_public: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RebateVector {
    pub gross_fee: String,
    pub rebate_amount: String,
    pub protocol_fee: String,
    pub bps_denom: String,
    pub rebate_bps: String,
    pub policy_version_private: String,
    pub policy_version_public: String,
    pub recipient_commitment_private: String,
    pub recipient_commitment_public: String,
    pub gross_fee_public: String,
    pub rebate_amount_public: String,
    pub protocol_fee_public: String,
}

pub fn validate_settlement_vector(v: &SettlementVector) -> Result<(), String> {
    let amount_in = parse_num(&v.amount_in)?;
    let amount_out = parse_num(&v.amount_out)?;
    let fee = parse_num(&v.fee)?;
    let execution_size = parse_num(&v.execution_size)?;
    let execution_price = parse_num(&v.execution_price)?;
    let limit_price = parse_num(&v.limit_price)?;
    let max_notional = parse_num(&v.max_notional)?;
    let notional = parse_num(&v.notional)?;

    let policy_version_private = parse_num(&v.policy_version_private)?;
    let receipt_hash_private = parse_num(&v.receipt_hash_private)?;
    let domain_separator_private = parse_num(&v.domain_separator_private)?;
    let workflow_run_id_private = parse_num(&v.workflow_run_id_private)?;
    let policy_version_public = parse_num(&v.policy_version_public)?;
    let receipt_hash_public = parse_num(&v.receipt_hash_public)?;
    let domain_separator_public = parse_num(&v.domain_separator_public)?;
    let workflow_run_id_public = parse_num(&v.workflow_run_id_public)?;
    let binding_hash_public = parse_num(&v.binding_hash_public)?;
    let notional_public = parse_num(&v.notional_public)?;

    if amount_in != (&amount_out + &fee) {
        return Err("conservation check failed: amount_in != amount_out + fee".to_string());
    }

    if notional != (&execution_size * &execution_price) {
        return Err("notional consistency failed: notional != execution_size * execution_price".to_string());
    }

    if notional != notional_public {
        return Err("public notional binding failed".to_string());
    }

    for (name, value) in [
        ("amount_in", &amount_in),
        ("amount_out", &amount_out),
        ("fee", &fee),
        ("execution_size", &execution_size),
        ("execution_price", &execution_price),
        ("limit_price", &limit_price),
        ("notional", &notional),
    ] {
        if value.is_zero() {
            return Err(format!("{name} must be > 0"));
        }
    }

    if notional > max_notional {
        return Err("max notional check failed".to_string());
    }

    if execution_price > limit_price {
        return Err("limit price check failed: execution_price > limit_price".to_string());
    }

    if policy_version_private != policy_version_public {
        return Err("policy version binding failed".to_string());
    }

    if receipt_hash_private != receipt_hash_public {
        return Err("receipt hash binding failed".to_string());
    }

    if domain_separator_private != domain_separator_public {
        return Err("domain separator binding failed".to_string());
    }
    if workflow_run_id_private != workflow_run_id_public {
        return Err("workflow run id binding failed".to_string());
    }

    let binding_calc = &workflow_run_id_public * BigUint::from(23u32)
        + &policy_version_public * BigUint::from(131u32)
        + &receipt_hash_public * BigUint::from(17u32)
        + &domain_separator_public * BigUint::from(19u32);

    if binding_calc != binding_hash_public {
        return Err("binding hash check failed".to_string());
    }

    Ok(())
}

pub fn validate_compliance_vector(v: &ComplianceVector) -> Result<(), String> {
    let compliance_pass_bit = parse_num(&v.compliance_pass_bit)?;
    let risk_score = parse_num(&v.risk_score)?;
    let max_risk_score = parse_num(&v.max_risk_score)?;
    let policy_version_private = parse_num(&v.policy_version_private)?;
    let policy_version_public = parse_num(&v.policy_version_public)?;
    let attestation_hash_private = parse_num(&v.attestation_hash_private)?;
    let attestation_hash_public = parse_num(&v.attestation_hash_public)?;
    let sanctions_commitment_private = parse_num(&v.sanctions_commitment_private)?;
    let sanctions_commitment_public = parse_num(&v.sanctions_commitment_public)?;
    let allowlist_commitment_private = parse_num(&v.allowlist_commitment_private)?;
    let allowlist_commitment_public = parse_num(&v.allowlist_commitment_public)?;

    if compliance_pass_bit != BigUint::one() {
        return Err("compliance pass bit must be 1".to_string());
    }

    if risk_score > max_risk_score {
        return Err("risk threshold check failed: risk_score > max_risk_score".to_string());
    }

    if policy_version_private != policy_version_public {
        return Err("policy version binding failed".to_string());
    }

    if attestation_hash_private != attestation_hash_public {
        return Err("attestation hash binding failed".to_string());
    }

    if sanctions_commitment_private != sanctions_commitment_public {
        return Err("sanctions commitment binding failed".to_string());
    }

    if allowlist_commitment_private != allowlist_commitment_public {
        return Err("allowlist commitment binding failed".to_string());
    }

    Ok(())
}

pub fn validate_rebate_vector(v: &RebateVector) -> Result<(), String> {
    let gross_fee = parse_num(&v.gross_fee)?;
    let rebate_amount = parse_num(&v.rebate_amount)?;
    let protocol_fee = parse_num(&v.protocol_fee)?;
    let bps_denom = parse_num(&v.bps_denom)?;
    let rebate_bps = parse_num(&v.rebate_bps)?;

    let policy_version_private = parse_num(&v.policy_version_private)?;
    let policy_version_public = parse_num(&v.policy_version_public)?;
    let recipient_commitment_private = parse_num(&v.recipient_commitment_private)?;
    let recipient_commitment_public = parse_num(&v.recipient_commitment_public)?;

    let gross_fee_public = parse_num(&v.gross_fee_public)?;
    let rebate_amount_public = parse_num(&v.rebate_amount_public)?;
    let protocol_fee_public = parse_num(&v.protocol_fee_public)?;

    if gross_fee != (&rebate_amount + &protocol_fee) {
        return Err("fee split check failed: gross_fee != rebate_amount + protocol_fee".to_string());
    }

    if (&rebate_amount * &bps_denom) != (&gross_fee * &rebate_bps) {
        return Err("rebate formula check failed".to_string());
    }

    for (name, value) in [
        ("gross_fee", &gross_fee),
        ("rebate_amount", &rebate_amount),
        ("protocol_fee", &protocol_fee),
        ("bps_denom", &bps_denom),
    ] {
        if value.is_zero() {
            return Err(format!("{name} must be > 0"));
        }
    }

    if rebate_amount > gross_fee {
        return Err("bound check failed: rebate_amount > gross_fee".to_string());
    }
    if protocol_fee > gross_fee {
        return Err("bound check failed: protocol_fee > gross_fee".to_string());
    }
    if rebate_bps > bps_denom {
        return Err("bound check failed: rebate_bps > bps_denom".to_string());
    }

    if policy_version_private != policy_version_public {
        return Err("policy version binding failed".to_string());
    }
    if recipient_commitment_private != recipient_commitment_public {
        return Err("recipient commitment binding failed".to_string());
    }

    if gross_fee != gross_fee_public
        || rebate_amount != rebate_amount_public
        || protocol_fee != protocol_fee_public
    {
        return Err("public fee field binding failed".to_string());
    }

    Ok(())
}

fn parse_num(s: &str) -> Result<BigUint, String> {
    if let Some(hex) = s.strip_prefix("0x") {
        return BigUint::parse_bytes(hex.as_bytes(), 16)
            .ok_or_else(|| format!("invalid hex number: {s}"));
    }
    BigUint::parse_bytes(s.as_bytes(), 10).ok_or_else(|| format!("invalid decimal number: {s}"))
}

pub fn max_field_for_nbits(n_bits: usize) -> BigUint {
    (BigUint::one() << n_bits) - BigUint::one()
}
