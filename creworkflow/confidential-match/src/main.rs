use std::io::{self, Read};

use confidential_match::errors::MatchError;
use confidential_match::handler::process_confidential_match;
use confidential_match::models::ConfidentialMatchRequest;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), MatchError> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| MatchError::InvalidRequest(format!("failed reading stdin: {e}")))?;

    let request: ConfidentialMatchRequest = serde_json::from_str(&input)
        .map_err(|e| MatchError::InvalidRequest(format!("invalid json input: {e}")))?;

    let response = process_confidential_match(request)?;
    let output = serde_json::to_string_pretty(&response)
        .map_err(|e| MatchError::InvalidRequest(format!("failed serializing output: {e}")))?;
    println!("{output}");
    Ok(())
}
