use crate::module::compliance_attestation::model::NormalizedSubject;
use crate::module::compliance_attestation::schema::ComplianceDecision;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn compute_attestation_hash(
    workflow_run_id: &str,
    request_id: &str,
    policy_version: &str,
    policy_hash: &str,
    decision: ComplianceDecision,
    risk_score: u16,
    issued_at: i64,
    expires_at: i64,
    sanctions_hit_count: usize,
    subjects: &[NormalizedSubject],
    match_digest: &str,
) -> String {
    let subjects_digest = digest_subjects(subjects);
    let canonical = CanonicalAttestationHashInput {
        workflow_run_id,
        request_id,
        policy_version,
        policy_hash,
        decision: decision.as_str(),
        risk_score,
        issued_at,
        expires_at,
        sanctions_hit_count,
        subjects_digest: &subjects_digest,
        match_digest,
    };

    let encoded = serde_json::to_vec(&canonical).expect("canonical serialization should not fail");
    let mut hasher = Sha256::new();
    hasher.update(&encoded);
    hex::encode(hasher.finalize())
}

pub fn build_attestation_id(attestation_hash: &str) -> String {
    let short = &attestation_hash[..24];
    format!("attn_{short}")
}

fn digest_subjects(subjects: &[NormalizedSubject]) -> String {
    let mut stable: Vec<CanonicalSubject> = subjects
        .iter()
        .map(|s| CanonicalSubject {
            subject_id: s.subject_id.clone(),
            subject_type: match s.subject_type {
                crate::module::compliance_attestation::model::SubjectType::Counterparty => "COUNTERPARTY",
                crate::module::compliance_attestation::model::SubjectType::Entity => "ENTITY",
            },
            jurisdiction: s.jurisdiction.clone(),
            address: s.address.clone(),
            legal_name: s.legal_name.clone(),
        })
        .collect();
    stable.sort_by(|a, b| a.subject_id.cmp(&b.subject_id).then(a.subject_type.cmp(b.subject_type)));

    let encoded = serde_json::to_vec(&stable).expect("canonical serialization should not fail");
    let mut hasher = Sha256::new();
    hasher.update(encoded);
    hex::encode(hasher.finalize())
}

#[derive(Serialize)]
struct CanonicalAttestationHashInput<'a> {
    workflow_run_id: &'a str,
    request_id: &'a str,
    policy_version: &'a str,
    policy_hash: &'a str,
    decision: &'a str,
    risk_score: u16,
    issued_at: i64,
    expires_at: i64,
    sanctions_hit_count: usize,
    subjects_digest: &'a str,
    match_digest: &'a str,
}

#[derive(Serialize)]
struct CanonicalSubject {
    subject_id: String,
    subject_type: &'static str,
    jurisdiction: Option<String>,
    address: Option<String>,
    legal_name: Option<String>,
}
