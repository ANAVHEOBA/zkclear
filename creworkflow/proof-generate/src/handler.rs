use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ethers_core::abi::{encode, Token};
use ethers_core::types::{H256, U256};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::errors::ProofError;
use crate::models::{ProofGenerateRequest, ProofGenerateResponse};

pub fn process_proof_generate(
    req: ProofGenerateRequest,
) -> Result<ProofGenerateResponse, ProofError> {
    validate_input(&req)?;
    enforce_timeout(&req)?;
    enforce_constraints(&req)?;

    let workflow_run_h256 = to_h256(&req.workflow_run_id);
    let domain_h256 = to_h256(&req.domain_separator);

    let receipt_hash = hash_hex(&[
        req.workflow_run_id.as_bytes(),
        req.settlement_params.asset_pair.as_bytes(),
        req.settlement_params.buy_intent_id.as_bytes(),
        req.settlement_params.sell_intent_id.as_bytes(),
        &req.policy_result.policy_version.to_be_bytes(),
        req.domain_separator.as_bytes(),
    ]);
    let receipt_h256 = to_h256(&receipt_hash);

    let workflow_run_id_public = low64_from_h256(&workflow_run_h256);
    let receipt_hash_public = low64_from_h256(&receipt_h256);
    let domain_separator_public = low64_from_h256(&domain_h256);
    let policy_version_public = req.policy_result.policy_version;

    let binding_hash_public = workflow_run_id_public as u128 * 23
        + policy_version_public as u128 * 131
        + receipt_hash_public as u128 * 17
        + domain_separator_public as u128 * 19;

    let circuit_input = build_circuit_input(
        &req,
        workflow_run_id_public,
        receipt_hash_public,
        domain_separator_public,
        policy_version_public,
        binding_hash_public,
    )?;

    let artifacts = resolve_artifacts()?;
    let run_id = unique_run_id();
    let tmp_dir = env::temp_dir().join("zkclear-proof-generate");
    fs::create_dir_all(&tmp_dir).map_err(|e| {
        ProofError::Artifact(format!(
            "failed to create temp dir `{}`: {e}",
            tmp_dir.display()
        ))
    })?;

    let input_file = tmp_dir.join(format!("input-{run_id}.json"));
    let wtns_file = tmp_dir.join(format!("witness-{run_id}.wtns"));
    let proof_file = tmp_dir.join(format!("proof-{run_id}.json"));
    let public_file = tmp_dir.join(format!("public-{run_id}.json"));

    fs::write(
        &input_file,
        serde_json::to_vec_pretty(&circuit_input)
            .map_err(|e| ProofError::Artifact(e.to_string()))?,
    )
    .map_err(|e| {
        ProofError::Artifact(format!("failed to write `{}`: {e}", input_file.display()))
    })?;

    run_snarkjs(&[
        "wtns",
        "calculate",
        artifacts
            .wasm
            .to_str()
            .ok_or_else(|| ProofError::Artifact("invalid wasm path".to_string()))?,
        input_file
            .to_str()
            .ok_or_else(|| ProofError::Artifact("invalid input path".to_string()))?,
        wtns_file
            .to_str()
            .ok_or_else(|| ProofError::Artifact("invalid wtns path".to_string()))?,
    ])?;

    run_snarkjs(&[
        "groth16",
        "prove",
        artifacts
            .zkey
            .to_str()
            .ok_or_else(|| ProofError::Artifact("invalid zkey path".to_string()))?,
        wtns_file
            .to_str()
            .ok_or_else(|| ProofError::Artifact("invalid wtns path".to_string()))?,
        proof_file
            .to_str()
            .ok_or_else(|| ProofError::Artifact("invalid proof path".to_string()))?,
        public_file
            .to_str()
            .ok_or_else(|| ProofError::Artifact("invalid public path".to_string()))?,
    ])?;

    let proof_json: SnarkProof = read_json(&proof_file)?;
    let public_signals: Vec<String> = read_json(&public_file)?;
    if public_signals.len() != 6 {
        return Err(ProofError::Artifact(format!(
            "unexpected public signal length: got {}, expected 6",
            public_signals.len()
        )));
    }

    let expected_signals = [
        policy_version_public.to_string(),
        receipt_hash_public.to_string(),
        domain_separator_public.to_string(),
        workflow_run_id_public.to_string(),
        binding_hash_public.to_string(),
        circuit_input["notional_public"]
            .as_str()
            .unwrap_or("0")
            .to_string(),
    ];

    for (idx, expected) in expected_signals.iter().enumerate() {
        if public_signals[idx] != *expected {
            return Err(ProofError::Artifact(format!(
                "public signal mismatch at index {idx}: expected {expected}, got {}",
                public_signals[idx]
            )));
        }
    }

    let proof_bytes = encode_proof_bytes(&proof_json)?;
    let proof_hash = hash_hex(&[&proof_bytes]);

    Ok(ProofGenerateResponse {
        proof_bytes,
        public_signals,
        proof_hash,
        receipt_hash,
        policy_version: req.policy_result.policy_version,
        domain_binding_hash: format!("0x{:x}", binding_hash_public),
    })
}

fn validate_input(req: &ProofGenerateRequest) -> Result<(), ProofError> {
    if req.workflow_run_id.trim().is_empty() {
        return Err(ProofError::InvalidRequest(
            "workflow_run_id cannot be empty".to_string(),
        ));
    }
    if req.domain_separator.trim().is_empty() {
        return Err(ProofError::InvalidRequest(
            "domain_separator cannot be empty".to_string(),
        ));
    }
    if req.witness_seed.trim().is_empty() {
        return Err(ProofError::WitnessGenerationFailure(
            "witness_seed missing".to_string(),
        ));
    }
    Ok(())
}

fn enforce_timeout(req: &ProofGenerateRequest) -> Result<(), ProofError> {
    if req.estimated_proving_time_ms > req.proving_timeout_ms {
        return Err(ProofError::ProvingTimeout {
            timeout_ms: req.proving_timeout_ms,
            required_ms: req.estimated_proving_time_ms,
        });
    }
    Ok(())
}

fn enforce_constraints(req: &ProofGenerateRequest) -> Result<(), ProofError> {
    if !req.match_result.accepted {
        return Err(ProofError::CircuitConstraintFailure(
            "match result not accepted".to_string(),
        ));
    }
    if !req.policy_result.passed {
        return Err(ProofError::CircuitConstraintFailure(
            "policy result not passed".to_string(),
        ));
    }
    if req.policy_result.policy_version == 0 {
        return Err(ProofError::CircuitConstraintFailure(
            "policy version must be non-zero".to_string(),
        ));
    }
    if req.settlement_params.execution_size <= 0.0 || req.settlement_params.execution_price <= 0.0 {
        return Err(ProofError::CircuitConstraintFailure(
            "execution size/price must be positive".to_string(),
        ));
    }

    let computed_notional =
        req.settlement_params.execution_size * req.settlement_params.execution_price;
    let delta = (computed_notional - req.settlement_params.notional).abs();
    if delta > 1e-8 {
        return Err(ProofError::CircuitConstraintFailure(
            "settlement notional mismatch".to_string(),
        ));
    }
    Ok(())
}

fn build_circuit_input(
    req: &ProofGenerateRequest,
    workflow_run_id_public: u64,
    receipt_hash_public: u64,
    domain_separator_public: u64,
    policy_version_public: u64,
    binding_hash_public: u128,
) -> Result<serde_json::Value, ProofError> {
    let execution_size = req.settlement_params.execution_size.round() as u64;
    let execution_price = req.settlement_params.execution_price.round() as u64;
    if execution_size == 0 || execution_price == 0 {
        return Err(ProofError::CircuitConstraintFailure(
            "execution size/price rounded to zero".to_string(),
        ));
    }
    let notional = execution_size
        .checked_mul(execution_price)
        .ok_or_else(|| ProofError::CircuitConstraintFailure("notional overflow".to_string()))?;

    let fee = 1u64;
    let amount_out = notional;
    let amount_in = amount_out + fee;
    let limit_price = execution_price + 100;
    let max_notional = notional + 1000;

    Ok(json!({
        "amount_in": amount_in.to_string(),
        "amount_out": amount_out.to_string(),
        "fee": fee.to_string(),
        "execution_size": execution_size.to_string(),
        "execution_price": execution_price.to_string(),
        "limit_price": limit_price.to_string(),
        "max_notional": max_notional.to_string(),
        "notional": notional.to_string(),
        "policy_version_private": policy_version_public.to_string(),
        "receipt_hash_private": receipt_hash_public.to_string(),
        "domain_separator_private": domain_separator_public.to_string(),
        "workflow_run_id_private": workflow_run_id_public.to_string(),
        "policy_version_public": policy_version_public.to_string(),
        "receipt_hash_public": receipt_hash_public.to_string(),
        "domain_separator_public": domain_separator_public.to_string(),
        "workflow_run_id_public": workflow_run_id_public.to_string(),
        "binding_hash_public": binding_hash_public.to_string(),
        "notional_public": notional.to_string()
    }))
}

fn run_snarkjs(args: &[&str]) -> Result<(), ProofError> {
    let bin = resolve_snarkjs_bin();
    let output = Command::new(&bin)
        .args(args)
        .output()
        .map_err(|e| ProofError::ProverCommand(format!("failed to execute `{bin}`: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ProofError::ProverCommand(format!(
            "`{bin} {}` failed: {}",
            args.join(" "),
            stderr.trim()
        )));
    }
    Ok(())
}

fn resolve_snarkjs_bin() -> String {
    if let Ok(bin) = env::var("SNARKJS_BIN") {
        if !bin.trim().is_empty() {
            return bin;
        }
    }
    let zk_root = env::var("ZK_ROOT").unwrap_or_else(|_| "../../zk".to_string());
    let local = PathBuf::from(zk_root).join("node_modules/.bin/snarkjs");
    if local.exists() {
        return local.to_string_lossy().into_owned();
    }
    "snarkjs".to_string()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ProofError> {
    let raw = fs::read_to_string(path)
        .map_err(|e| ProofError::Artifact(format!("failed to read `{}`: {e}", path.display())))?;
    serde_json::from_str(&raw)
        .map_err(|e| ProofError::Artifact(format!("invalid json `{}`: {e}", path.display())))
}

fn hash_hex(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    format!("0x{}", hex::encode(hasher.finalize()))
}

fn to_h256(input: &str) -> H256 {
    if let Ok(parsed) = input.parse::<H256>() {
        return parsed;
    }
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    H256::from_slice(&hasher.finalize())
}

fn low64_from_h256(value: &H256) -> u64 {
    let bytes = value.as_bytes();
    let mut low = [0u8; 8];
    low.copy_from_slice(&bytes[24..32]);
    u64::from_be_bytes(low)
}

fn unique_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}")
}

fn encode_proof_bytes(proof: &SnarkProof) -> Result<Vec<u8>, ProofError> {
    if proof.pi_a.len() < 2 || proof.pi_b.len() < 2 || proof.pi_c.len() < 2 {
        return Err(ProofError::Artifact(
            "proof json missing coordinates".to_string(),
        ));
    }
    if proof.pi_b[0].len() < 2 || proof.pi_b[1].len() < 2 {
        return Err(ProofError::Artifact(
            "proof json missing pi_b coordinates".to_string(),
        ));
    }

    let p_a = vec![parse_u256(&proof.pi_a[0])?, parse_u256(&proof.pi_a[1])?];
    let p_b = vec![
        vec![
            parse_u256(&proof.pi_b[0][1])?,
            parse_u256(&proof.pi_b[0][0])?,
        ],
        vec![
            parse_u256(&proof.pi_b[1][1])?,
            parse_u256(&proof.pi_b[1][0])?,
        ],
    ];
    let p_c = vec![parse_u256(&proof.pi_c[0])?, parse_u256(&proof.pi_c[1])?];

    Ok(encode(&[
        Token::FixedArray(vec![Token::Uint(p_a[0]), Token::Uint(p_a[1])]),
        Token::FixedArray(vec![
            Token::FixedArray(vec![Token::Uint(p_b[0][0]), Token::Uint(p_b[0][1])]),
            Token::FixedArray(vec![Token::Uint(p_b[1][0]), Token::Uint(p_b[1][1])]),
        ]),
        Token::FixedArray(vec![Token::Uint(p_c[0]), Token::Uint(p_c[1])]),
    ]))
}

fn parse_u256(s: &str) -> Result<U256, ProofError> {
    if let Some(hex) = s.strip_prefix("0x") {
        return U256::from_str_radix(hex, 16)
            .map_err(|e| ProofError::Artifact(format!("invalid hex u256 `{s}`: {e}")));
    }
    U256::from_dec_str(s)
        .map_err(|e| ProofError::Artifact(format!("invalid decimal u256 `{s}`: {e}")))
}

struct ArtifactPaths {
    wasm: PathBuf,
    zkey: PathBuf,
}

fn resolve_artifacts() -> Result<ArtifactPaths, ProofError> {
    let zk_root = env::var("ZK_ROOT").unwrap_or_else(|_| "../../zk".to_string());
    let root = PathBuf::from(zk_root);
    let wasm = root.join("artifacts/settlement_valid/settlement_valid_js/settlement_valid.wasm");
    let zkey = root.join("artifacts/settlement_valid/settlement_valid.zkey");
    if !wasm.exists() {
        return Err(ProofError::Artifact(format!(
            "missing wasm artifact `{}`",
            wasm.display()
        )));
    }
    if !zkey.exists() {
        return Err(ProofError::Artifact(format!(
            "missing zkey artifact `{}`",
            zkey.display()
        )));
    }
    Ok(ArtifactPaths { wasm, zkey })
}

#[derive(Debug, Deserialize)]
struct SnarkProof {
    pi_a: Vec<String>,
    pi_b: Vec<Vec<String>>,
    pi_c: Vec<String>,
}
