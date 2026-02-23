use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProofError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("circuit constraint failure: {0}")]
    CircuitConstraintFailure(String),

    #[error("witness generation failure: {0}")]
    WitnessGenerationFailure(String),

    #[error("proving timeout: timeout_ms={timeout_ms}, required_ms={required_ms}")]
    ProvingTimeout { timeout_ms: u64, required_ms: u64 },

    #[error("prover command failed: {0}")]
    ProverCommand(String),

    #[error("artifact error: {0}")]
    Artifact(String),
}
