use crate::client::cache::{CacheTtl, EdgarCache};
use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::cik::Cik;
use crate::models::filing::Filing;
use crate::models::submission::{RecentFilings, SubmissionsResponse};

const BASE: &str = "https://data.sec.gov/submissions";

/// Fetch the full submissions response for a CIK.
pub async fn fetch_submissions(
    http: &RateLimitedHttp,
    cache: &EdgarCache,
    cik: Cik,
) -> Result<SubmissionsResponse> {
    let url = format!("{}/CIK{}.json", BASE, cik.zero_padded());
    let cache_key = format!("submissions:{}", cik.as_u64());

    if let Some(cached) = cache.get(&cache_key).await {
        return Ok(serde_json::from_str(&cached)?);
    }

    let body = http.get_text(&url).await?;
    cache
        .insert(cache_key, body.clone(), CacheTtl::SUBMISSIONS)
        .await;

    Ok(serde_json::from_str(&body)?)
}

/// Fetch all filings for a CIK, including paginated older filings.
pub async fn fetch_filings(
    http: &RateLimitedHttp,
    cache: &EdgarCache,
    cik: Cik,
) -> Result<Vec<Filing>> {
    let resp = fetch_submissions(http, cache, cik).await?;
    let mut filings = resp.filings.recent.to_filings();

    // Fetch paginated older filings
    for file in &resp.filings.files {
        let url = format!("{}/{}", BASE, file.name);
        let body = http.get_text(&url).await?;
        let older: RecentFilings = serde_json::from_str(&body)?;
        filings.extend(older.to_filings());
    }

    Ok(filings)
}
