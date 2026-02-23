use thiserror::Error;

#[derive(Debug, Error)]
pub enum PublishError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("unauthorized caller")]
    UnauthorizedCaller,

    #[error("stale or inactive policy version")]
    StalePolicy,

    #[error("invalid proof")]
    InvalidProof,

    #[error("invalid signal binding")]
    InvalidSignalBinding,

    #[error("duplicate workflow run")]
    DuplicateWorkflowRun,

    #[error("duplicate receipt hash")]
    DuplicateReceiptHash,

    #[error("missing environment variable: {0}")]
    MissingEnv(String),

    #[error("onchain integration error: {0}")]
    Onchain(String),
}
