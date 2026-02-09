use crate::client::cache::{CacheTtl, EdgarCache};
use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::frame::FrameResponse;
use crate::models::period::CalendarPeriod;

const BASE: &str = "https://data.sec.gov/api/xbrl/frames";

/// Fetch cross-company XBRL frame data.
///
/// Example: `fetch_frame(http, cache, "us-gaap", "Revenues", "USD", &CalendarPeriod::annual(2023))`
pub async fn fetch_frame(
    http: &RateLimitedHttp,
    cache: &EdgarCache,
    taxonomy: &str,
    tag: &str,
    unit: &str,
    period: &CalendarPeriod,
) -> Result<FrameResponse> {
    let url = format!("{}/{}/{}/{}/{}.json", BASE, taxonomy, tag, unit, period);
    let cache_key = format!("frame:{}:{}:{}:{}", taxonomy, tag, unit, period);

    if let Some(cached) = cache.get(&cache_key).await {
        return Ok(serde_json::from_str(&cached)?);
    }

    let body = http.get_text(&url).await?;
    cache
        .insert(cache_key, body.clone(), CacheTtl::FRAMES)
        .await;

    Ok(serde_json::from_str(&body)?)
}
