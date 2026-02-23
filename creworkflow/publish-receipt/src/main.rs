use std::io::{self, Read};

use publish_receipt::chain::{process_publish_receipt_onchain, ChainConfig};
use publish_receipt::errors::PublishError;
use publish_receipt::handler::process_publish_receipt;
use publish_receipt::models::PublishReceiptRequest;
use tokio::runtime::Runtime;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), PublishError> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| PublishError::InvalidRequest(format!("failed reading stdin: {e}")))?;

    let request: PublishReceiptRequest =
        serde_json::from_str(&input).map_err(|e| PublishError::InvalidRequest(format!("invalid json input: {e}")))?;

    let response = if let Some(cfg) = read_chain_config()? {
        let rt = Runtime::new().map_err(|e| PublishError::Onchain(format!("runtime init failed: {e}")))?;
        rt.block_on(process_publish_receipt_onchain(request, cfg))?
    } else {
        process_publish_receipt(request)?
    };

    let output = serde_json::to_string_pretty(&response)
        .map_err(|e| PublishError::InvalidRequest(format!("failed serializing output: {e}")))?;
    println!("{output}");
    Ok(())
}

fn read_chain_config() -> Result<Option<ChainConfig>, PublishError> {
    let rpc = std::env::var("ETH_SEPOLIA_RPC_URL").ok();
    let pk = std::env::var("PRIVATE_KEY").ok();

    match (rpc, pk) {
        (Some(rpc_url), Some(private_key)) => {
            let chain_id = std::env::var("ETH_SEPOLIA_CHAIN_ID")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(11155111);
            Ok(Some(ChainConfig {
                rpc_url,
                private_key,
                chain_id,
            }))
        }
        (None, None) => Ok(None),
        _ => Err(PublishError::MissingEnv(
            "set both ETH_SEPOLIA_RPC_URL and PRIVATE_KEY, or set neither for simulation mode".to_string(),
        )),
    }
}
