use std::io::{self, Read};

use settle_private::errors::SettleError;
use settle_private::handler::process_settle_private;
use settle_private::models::SettlePrivateRequest;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), SettleError> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| SettleError::InvalidRequest(format!("failed reading stdin: {e}")))?;

    let request: SettlePrivateRequest =
        serde_json::from_str(&input).map_err(|e| SettleError::InvalidRequest(format!("invalid json input: {e}")))?;

    let response = process_settle_private(request)?;
    let output = serde_json::to_string_pretty(&response)
        .map_err(|e| SettleError::InvalidRequest(format!("failed serializing output: {e}")))?;
    println!("{output}");
    Ok(())
}
