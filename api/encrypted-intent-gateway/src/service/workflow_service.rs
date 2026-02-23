pub fn generate_workflow_run_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
