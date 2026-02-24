use crate::app::AppState;
use crate::module::proof_job::model::{ProofJobRecord, ProverArtifactsRecord};
use crate::module::proof_job::schema::ProofType;
use crate::service::binding_codec_service::settlement_binding_fields;
use crate::service::hash_service::sha256_hex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

pub async fn run_prover_for_job(
    state: &AppState,
    job: &ProofJobRecord,
) -> Result<ProverArtifactsRecord, String> {
    let circuit = map_circuit(&job.proof_type);
    let zk_root = resolve_zk_root(&state.config.zk_root_dir)?;
    let base_fixture_path = resolve_fixture_path(&zk_root, &job.receipt_context, circuit);
    let fixture_path = if matches!(job.proof_type, ProofType::Settlement) {
        prepare_runtime_settlement_fixture(&base_fixture_path, state, job).await?
    } else {
        base_fixture_path
    };

    let prove_script = zk_root.join("scripts").join("prove.sh");
    let timeout_secs = state.config.prove_timeout_seconds.max(1) as u64;
    let budget_secs = budget_for_type(state, &job.proof_type).max(1);

    let started = Instant::now();
    let child = Command::new("bash")
        .arg(prove_script.as_os_str())
        .arg(circuit)
        .arg(fixture_path.as_os_str())
        .current_dir(&zk_root)
        .output();

    let output = timeout(Duration::from_secs(timeout_secs), child)
        .await
        .map_err(|_| format!("prove timeout after {timeout_secs}s"))?
        .map_err(|e| format!("prove command failed to start: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "prove command failed (status={}): stdout={} stderr={}",
            output.status,
            trim_log(&stdout),
            trim_log(&stderr)
        ));
    }

    let prove_time_seconds = started.elapsed().as_secs() as i64;
    if prove_time_seconds > budget_secs {
        return Err(format!(
            "prove time budget exceeded: {}s > {}s",
            prove_time_seconds, budget_secs
        ));
    }

    let proof_path = zk_root
        .join("artifacts")
        .join(circuit)
        .join(format!("{circuit}.proof.json"));
    let public_path = zk_root
        .join("artifacts")
        .join(circuit)
        .join(format!("{circuit}.public.json"));

    let proof_raw = tokio::fs::read_to_string(&proof_path)
        .await
        .map_err(|e| format!("failed to read proof file: {e}"))?;
    let public_raw = tokio::fs::read_to_string(&public_path)
        .await
        .map_err(|e| format!("failed to read public file: {e}"))?;
    let proof_json: Value =
        serde_json::from_str(&proof_raw).map_err(|e| format!("proof.json parse failed: {e}"))?;
    let public_json: Value =
        serde_json::from_str(&public_raw).map_err(|e| format!("public.json parse failed: {e}"))?;

    let proof_hash = sha256_hex(&proof_raw);
    let receipt_hash = resolve_receipt_hash(&job.receipt_context, &public_raw)?;

    Ok(ProverArtifactsRecord {
        circuit: circuit.to_string(),
        fixture_path: path_to_string(&fixture_path),
        proof_path: path_to_string(&proof_path),
        public_path: path_to_string(&public_path),
        proof_json,
        public_json,
        proof_hash,
        receipt_hash,
        prove_time_seconds,
    })
}

fn map_circuit(proof_type: &ProofType) -> &'static str {
    match proof_type {
        ProofType::Settlement => "settlement_valid",
        ProofType::Compliance => "compliance_valid",
        ProofType::Rebate => "rebate_valid",
    }
}

fn budget_for_type(state: &AppState, proof_type: &ProofType) -> i64 {
    match proof_type {
        ProofType::Settlement => state.config.prove_budget_settlement_seconds,
        ProofType::Compliance => state.config.prove_budget_compliance_seconds,
        ProofType::Rebate => state.config.prove_budget_rebate_seconds,
    }
}

fn resolve_zk_root(config_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(config_path);
    if path.is_absolute() {
        return Ok(path);
    }
    let cwd = std::env::current_dir().map_err(|e| format!("current dir error: {e}"))?;
    Ok(cwd.join(path))
}

fn resolve_fixture_path(zk_root: &Path, receipt_context: &Value, circuit: &str) -> PathBuf {
    let configured = receipt_context
        .get("fixturePath")
        .or_else(|| receipt_context.get("fixture_path"))
        .and_then(Value::as_str)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    match configured {
        Some(p) if p.is_absolute() => p,
        Some(p) => zk_root.join(p),
        None => zk_root
            .join("fixtures")
            .join(format!("{circuit}.fixture.json")),
    }
}

fn resolve_receipt_hash_from_context(receipt_context: &Value) -> Option<String> {
    if let Some(s) = receipt_context
        .get("receiptHash")
        .or_else(|| receipt_context.get("receipt_hash"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return Some(s.to_string());
    }
    None
}

fn resolve_receipt_hash(receipt_context: &Value, public_raw: &str) -> Result<String, String> {
    if let Some(v) = resolve_receipt_hash_from_context(receipt_context) {
        return Ok(v);
    }
    Ok(sha256_hex(public_raw))
}

async fn prepare_runtime_settlement_fixture(
    base_fixture_path: &Path,
    state: &AppState,
    job: &ProofJobRecord,
) -> Result<PathBuf, String> {
    let receipt_hash = resolve_receipt_hash_from_context(&job.receipt_context)
        .unwrap_or_else(|| sha256_hex(&job.workflow_run_id));
    let domain_separator = resolve_binding_value(&job.receipt_context, "domainSeparator")
        .unwrap_or_else(|| state.config.signal_domain_separator.clone());
    let fields = settlement_binding_fields(
        &job.workflow_run_id,
        &job.policy_version,
        &receipt_hash,
        &domain_separator,
    );

    let raw = tokio::fs::read_to_string(base_fixture_path)
        .await
        .map_err(|e| format!("failed to read settlement fixture: {e}"))?;
    let mut fixture: Value =
        serde_json::from_str(&raw).map_err(|e| format!("fixture json parse failed: {e}"))?;
    let obj = fixture
        .as_object_mut()
        .ok_or_else(|| "settlement fixture must be a JSON object".to_string())?;

    obj.insert(
        "workflow_run_id_private".to_string(),
        Value::String(fields.workflow_run_id.clone()),
    );
    obj.insert(
        "workflow_run_id_public".to_string(),
        Value::String(fields.workflow_run_id),
    );
    obj.insert(
        "policy_version_private".to_string(),
        Value::String(fields.policy_version.clone()),
    );
    obj.insert(
        "policy_version_public".to_string(),
        Value::String(fields.policy_version),
    );
    obj.insert(
        "receipt_hash_private".to_string(),
        Value::String(fields.receipt_hash.clone()),
    );
    obj.insert(
        "receipt_hash_public".to_string(),
        Value::String(fields.receipt_hash),
    );
    obj.insert(
        "domain_separator_private".to_string(),
        Value::String(fields.domain_separator.clone()),
    );
    obj.insert(
        "domain_separator_public".to_string(),
        Value::String(fields.domain_separator),
    );
    obj.insert(
        "binding_hash_public".to_string(),
        Value::String(fields.binding_hash),
    );

    let mut out_path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    out_path.push(format!("settlement_runtime_fixture_{nanos}.json"));
    let encoded = serde_json::to_string(&fixture)
        .map_err(|e| format!("failed to encode settlement runtime fixture: {e}"))?;
    tokio::fs::write(&out_path, encoded)
        .await
        .map_err(|e| format!("failed to write settlement runtime fixture: {e}"))?;
    Ok(out_path)
}

fn resolve_binding_value(context: &Value, key: &str) -> Option<String> {
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

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn trim_log(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.len() > 400 {
        format!("{}...", &trimmed[..400])
    } else {
        trimmed.to_string()
    }
}
