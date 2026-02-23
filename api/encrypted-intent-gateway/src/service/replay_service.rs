use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::sync::Mutex;

static NONCES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static INTENT_HASHES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

pub fn check_replay(nonce: &str, intent_hash: &str) -> Result<(), String> {
    let mut nonces = NONCES
        .lock()
        .map_err(|_| "replay lock poisoned for nonces".to_string())?;
    let mut hashes = INTENT_HASHES
        .lock()
        .map_err(|_| "replay lock poisoned for hashes".to_string())?;

    if nonces.contains(nonce) {
        return Err("replay detected: nonce already used".to_string());
    }
    if hashes.contains(intent_hash) {
        return Err("replay detected: intent hash already used".to_string());
    }

    nonces.insert(nonce.to_string());
    hashes.insert(intent_hash.to_string());
    Ok(())
}

#[cfg(test)]
pub fn clear_replay_cache() {
    if let Ok(mut nonces) = NONCES.lock() {
        nonces.clear();
    }
    if let Ok(mut hashes) = INTENT_HASHES.lock() {
        hashes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn replay_rejects_duplicate_nonce() {
        let _guard = TEST_LOCK.lock().expect("lock");
        clear_replay_cache();
        check_replay("nonce-test-1", "hash-test-1").expect("first insert");
        let err = check_replay("nonce-test-1", "hash-test-2").expect_err("must reject duplicate nonce");
        assert!(err.contains("nonce already used"));
    }

    #[test]
    fn replay_rejects_duplicate_hash() {
        let _guard = TEST_LOCK.lock().expect("lock");
        clear_replay_cache();
        check_replay("nonce-test-3", "hash-test-3").expect("first insert");
        let err = check_replay("nonce-test-4", "hash-test-3").expect_err("must reject duplicate hash");
        assert!(err.contains("intent hash already used"));
    }
}
