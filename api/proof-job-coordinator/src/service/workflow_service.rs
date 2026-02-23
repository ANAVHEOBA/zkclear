use uuid::Uuid;

pub fn generate_job_id() -> String {
    format!("proofjob-{}", Uuid::new_v4())
}
