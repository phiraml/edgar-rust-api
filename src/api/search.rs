use crate::client::cache::{CacheTtl, EdgarCache};
use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::search::SearchResponse;

const EFTS_BASE: &str = "https://efts.sec.gov/LATEST/search-index";

/// Full-text search over EDGAR filings using the EFTS API.
pub async fn efts_search(
    http: &RateLimitedHttp,
    cache: &EdgarCache,
    query: &str,
    forms: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
    start: u32,
) -> Result<SearchResponse> {
    let mut url = format!("{}?q={}&from={}", EFTS_BASE, urlencoded(query), start);

    if let Some(forms) = forms {
        url.push_str(&format!("&forms={}", urlencoded(forms)));
    }
    if let Some(sd) = start_date {
        url.push_str(&format!("&startdt={}", sd));
    }
    if let Some(ed) = end_date {
        url.push_str(&format!("&enddt={}", ed));
    }

    let cache_key = format!("search:{}", url);

    if let Some(cached) = cache.get(&cache_key).await {
        return Ok(serde_json::from_str(&cached)?);
    }

    let body = http.get_text(&url).await?;
    cache
        .insert(cache_key, body.clone(), CacheTtl::SEARCH)
        .await;

    Ok(serde_json::from_str(&body)?)
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('"', "%22")
        .replace('&', "%26")
}
