use std::io::{self, Read};

use proof_generate::errors::ProofError;
use proof_generate::handler::process_proof_generate;
use proof_generate::models::ProofGenerateRequest;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), ProofError> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| ProofError::InvalidRequest(format!("failed reading stdin: {e}")))?;

    let request: ProofGenerateRequest =
        serde_json::from_str(&input).map_err(|e| ProofError::InvalidRequest(format!("invalid json input: {e}")))?;

    let response = process_proof_generate(request)?;
    let output = serde_json::to_string_pretty(&response)
        .map_err(|e| ProofError::InvalidRequest(format!("failed serializing output: {e}")))?;
    println!("{output}");
    Ok(())
}
