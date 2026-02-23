use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static INTENT_SUBMIT_SUCCESS: AtomicU64 = AtomicU64::new(0);
static INTENT_SUBMIT_FAILURE: AtomicU64 = AtomicU64::new(0);

pub fn record_intent_submit_success() {
    INTENT_SUBMIT_SUCCESS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_intent_submit_failure() {
    INTENT_SUBMIT_FAILURE.fetch_add(1, Ordering::Relaxed);
}

pub fn start_timer() -> Instant {
    Instant::now()
}

pub fn elapsed_ms(start: Instant) -> u128 {
    start.elapsed().as_millis()
}

pub fn snapshot() -> (u64, u64) {
    (
        INTENT_SUBMIT_SUCCESS.load(Ordering::Relaxed),
        INTENT_SUBMIT_FAILURE.load(Ordering::Relaxed),
    )
}
