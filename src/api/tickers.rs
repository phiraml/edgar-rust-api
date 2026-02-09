use std::collections::HashMap;

use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::ticker::{CompanyTicker, TickerMap};

const TICKERS_URL: &str = "https://www.sec.gov/files/company_tickers.json";

/// Fetch the ticker-to-CIK mapping from SEC.
pub async fn fetch_ticker_map(http: &RateLimitedHttp) -> Result<TickerMap> {
    let body = http.get_text(TICKERS_URL).await?;
    let raw: HashMap<String, CompanyTicker> = serde_json::from_str(&body)?;
    let entries: Vec<CompanyTicker> = raw.into_values().collect();
    Ok(TickerMap::from_entries(entries))
}
