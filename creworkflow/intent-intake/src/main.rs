use std::io::{self, Read};

use intent_intake::errors::IntakeError;
use intent_intake::handler::process_intake;
use intent_intake::models::IntentIntakeRequest;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), IntakeError> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| IntakeError::InvalidRequest(format!("failed reading stdin: {e}")))?;

    let request: IntentIntakeRequest = serde_json::from_str(&input)
        .map_err(|e| IntakeError::InvalidRequest(format!("invalid json input: {e}")))?;

    let response = process_intake(request)?;
    let output = serde_json::to_string_pretty(&response)
        .map_err(|e| IntakeError::InvalidRequest(format!("failed serializing output: {e}")))?;

    println!("{output}");
    Ok(())
}
