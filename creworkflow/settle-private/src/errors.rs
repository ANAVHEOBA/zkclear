use thiserror::Error;

#[derive(Debug, Error)]
pub enum SettleError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("transfer failure")]
    TransferFailure,

    #[error("counterparty settlement conflict")]
    CounterpartySettlementConflict,

    #[error("timeout/retry exhausted: attempts={attempts}, max_retries={max_retries}")]
    TimeoutRetryExhausted { attempts: u32, max_retries: u32 },
}
