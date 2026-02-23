use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

static JOBS_QUEUED: AtomicU64 = AtomicU64::new(0);
static JOBS_PUBLISHED: AtomicU64 = AtomicU64::new(0);
static JOBS_FAILED: AtomicU64 = AtomicU64::new(0);
static RETRY_SCHEDULED: AtomicU64 = AtomicU64::new(0);

static PROVE_DURATION_COUNT: AtomicU64 = AtomicU64::new(0);
static PROVE_DURATION_TOTAL_MS: AtomicU64 = AtomicU64::new(0);

static QUEUE_LATENCY_COUNT: AtomicU64 = AtomicU64::new(0);
static QUEUE_LATENCY_TOTAL_MS: AtomicU64 = AtomicU64::new(0);

static LAST_ERROR_TS: AtomicI64 = AtomicI64::new(0);

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub jobs_queued: u64,
    pub jobs_published: u64,
    pub jobs_failed: u64,
    pub retries_scheduled: u64,
    pub prove_duration_count: u64,
    pub prove_duration_avg_ms: u64,
    pub queue_latency_count: u64,
    pub queue_latency_avg_ms: u64,
    pub last_error_ts: i64,
}

pub fn inc_jobs_queued() {
    JOBS_QUEUED.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_jobs_published() {
    JOBS_PUBLISHED.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_jobs_failed() {
    JOBS_FAILED.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_retries_scheduled() {
    RETRY_SCHEDULED.fetch_add(1, Ordering::Relaxed);
}

pub fn record_prove_duration_ms(duration_ms: u64) {
    PROVE_DURATION_COUNT.fetch_add(1, Ordering::Relaxed);
    PROVE_DURATION_TOTAL_MS.fetch_add(duration_ms, Ordering::Relaxed);
}

pub fn record_queue_latency_ms(duration_ms: u64) {
    QUEUE_LATENCY_COUNT.fetch_add(1, Ordering::Relaxed);
    QUEUE_LATENCY_TOTAL_MS.fetch_add(duration_ms, Ordering::Relaxed);
}

pub fn set_last_error_ts(ts: i64) {
    LAST_ERROR_TS.store(ts, Ordering::Relaxed);
}

pub fn snapshot() -> MetricsSnapshot {
    let prove_count = PROVE_DURATION_COUNT.load(Ordering::Relaxed);
    let queue_count = QUEUE_LATENCY_COUNT.load(Ordering::Relaxed);

    MetricsSnapshot {
        jobs_queued: JOBS_QUEUED.load(Ordering::Relaxed),
        jobs_published: JOBS_PUBLISHED.load(Ordering::Relaxed),
        jobs_failed: JOBS_FAILED.load(Ordering::Relaxed),
        retries_scheduled: RETRY_SCHEDULED.load(Ordering::Relaxed),
        prove_duration_count: prove_count,
        prove_duration_avg_ms: if prove_count > 0 {
            PROVE_DURATION_TOTAL_MS.load(Ordering::Relaxed) / prove_count
        } else {
            0
        },
        queue_latency_count: queue_count,
        queue_latency_avg_ms: if queue_count > 0 {
            QUEUE_LATENCY_TOTAL_MS.load(Ordering::Relaxed) / queue_count
        } else {
            0
        },
        last_error_ts: LAST_ERROR_TS.load(Ordering::Relaxed),
    }
}
