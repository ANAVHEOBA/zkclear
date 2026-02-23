use crate::module::compliance_attestation::model::NormalizedSubject;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsEntry {
    pub source: String,
    pub program: String,
    pub name: String,
    pub jurisdiction: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningHit {
    pub entry_name: String,
    pub confidence: u8,
}

#[derive(Debug, Clone)]
pub struct ScreeningResult {
    pub hits: Vec<ScreeningHit>,
    pub match_digest: String,
}

pub fn load_sanctions_entries(path: &str) -> Result<Vec<SanctionsEntry>, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("failed to read sanctions file: {e}"))?;
    serde_json::from_str::<Vec<SanctionsEntry>>(&raw)
        .map_err(|e| format!("failed to parse sanctions file: {e}"))
}

pub fn screen_subjects(subjects: &[NormalizedSubject], entries: &[SanctionsEntry]) -> ScreeningResult {
    let mut hits = Vec::new();

    for subject in subjects {
        let candidates = candidate_strings(subject);
        for entry in entries {
            let entry_name = normalize(&entry.name);
            let mut best = 0u8;

            for candidate in &candidates {
                if candidate.is_empty() {
                    continue;
                }
                if *candidate == entry_name {
                    best = best.max(100);
                    continue;
                }
                if candidate.contains(&entry_name) || entry_name.contains(candidate) {
                    best = best.max(80);
                }
            }

            if best > 0 {
                hits.push(ScreeningHit {
                    entry_name: entry.name.clone(),
                    confidence: best,
                });
            }
        }
    }

    let match_digest = compute_match_digest(&hits);
    ScreeningResult { hits, match_digest }
}

fn compute_match_digest(hits: &[ScreeningHit]) -> String {
    let mut stable = hits
        .iter()
        .map(|h| format!("{}:{}", h.entry_name, h.confidence))
        .collect::<Vec<_>>();
    stable.sort();

    let mut hasher = Sha256::new();
    for line in stable {
        hasher.update(line.as_bytes());
        hasher.update(b"|");
    }
    hex::encode(hasher.finalize())
}

fn candidate_strings(subject: &NormalizedSubject) -> Vec<String> {
    vec![
        normalize(&subject.subject_id),
        normalize(subject.legal_name.as_deref().unwrap_or_default()),
        normalize(subject.address.as_deref().unwrap_or_default()),
    ]
}

fn normalize(input: &str) -> String {
    input
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
