use crate::client::cache::{CacheTtl, EdgarCache};
use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::cik::Cik;
use crate::models::company_facts::CompanyFactsResponse;

const BASE: &str = "https://data.sec.gov/api/xbrl/companyfacts";

/// Fetch all XBRL facts for a company.
pub async fn fetch_company_facts(
    http: &RateLimitedHttp,
    cache: &EdgarCache,
    cik: Cik,
) -> Result<CompanyFactsResponse> {
    let url = format!("{}/CIK{}.json", BASE, cik.zero_padded());
    let cache_key = format!("company_facts:{}", cik.as_u64());

    if let Some(cached) = cache.get(&cache_key).await {
        return Ok(serde_json::from_str(&cached)?);
    }

    let body = http.get_text(&url).await?;
    cache
        .insert(cache_key, body.clone(), CacheTtl::COMPANY_FACTS)
        .await;

    Ok(serde_json::from_str(&body)?)
}
