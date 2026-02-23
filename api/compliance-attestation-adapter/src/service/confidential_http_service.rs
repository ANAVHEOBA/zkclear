use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxQuote {
    pub provider: String,
    pub base: String,
    pub quote: String,
    pub rate: f64,
    pub as_of_date: String,
}

#[derive(Debug, Deserialize)]
struct FrankfurterLatestResponse {
    date: String,
    rates: std::collections::HashMap<String, f64>,
}

pub async fn fetch_fx_quote(
    base_url: &str,
    base_currency: &str,
    quote_currency: &str,
) -> Result<FxQuote, String> {
    let endpoint = format!(
        "{}/latest?base={}&symbols={}",
        base_url.trim_end_matches('/'),
        base_currency.to_uppercase(),
        quote_currency.to_uppercase()
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("failed to build http client: {e}"))?;

    let resp = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| format!("fx provider request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "fx provider returned non-success status: {}",
            resp.status()
        ));
    }

    let payload = resp
        .json::<FrankfurterLatestResponse>()
        .await
        .map_err(|e| format!("failed to parse fx provider payload: {e}"))?;

    let quote = quote_currency.to_uppercase();
    let rate = payload
        .rates
        .get(&quote)
        .copied()
        .ok_or_else(|| format!("fx provider payload missing rate for {quote}"))?;

    Ok(FxQuote {
        provider: "frankfurter".to_string(),
        base: base_currency.to_uppercase(),
        quote,
        rate,
        as_of_date: payload.date,
    })
}
