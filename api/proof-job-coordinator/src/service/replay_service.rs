pub fn replay_run_key(workflow_run_id: &str, proof_type: &str) -> String {
    format!("{}:{}", workflow_run_id.trim(), proof_type.trim())
}
