use crate::client::cache::{CacheTtl, EdgarCache};
use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::cik::Cik;
use crate::models::company_concept::CompanyConceptResponse;

const BASE: &str = "https://data.sec.gov/api/xbrl/companyconcept";

/// Fetch a specific XBRL concept for a company.
pub async fn fetch_company_concept(
    http: &RateLimitedHttp,
    cache: &EdgarCache,
    cik: Cik,
    taxonomy: &str,
    tag: &str,
) -> Result<CompanyConceptResponse> {
    let url = format!(
        "{}/CIK{}/{}/{}.json",
        BASE,
        cik.zero_padded(),
        taxonomy,
        tag
    );
    let cache_key = format!("concept:{}:{}:{}", cik.as_u64(), taxonomy, tag);

    if let Some(cached) = cache.get(&cache_key).await {
        return Ok(serde_json::from_str(&cached)?);
    }

    let body = http.get_text(&url).await?;
    cache
        .insert(cache_key, body.clone(), CacheTtl::COMPANY_CONCEPT)
        .await;

    Ok(serde_json::from_str(&body)?)
}
