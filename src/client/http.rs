use std::num::NonZeroU32;
use std::sync::Arc;

use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

use crate::error::{EdgarError, Result};

type GovernorLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Rate-limited HTTP client for EDGAR APIs.
///
/// Enforces the SEC's 10 requests/second limit via a token-bucket rate limiter.
#[derive(Clone)]
pub struct RateLimitedHttp {
    client: reqwest::Client,
    limiter: Arc<GovernorLimiter>,
}

impl RateLimitedHttp {
    /// Create a new rate-limited HTTP client.
    ///
    /// `user_agent` should identify your application (SEC requires a User-Agent header).
    /// `requests_per_second` defaults to 10 (SEC limit).
    pub fn new(user_agent: &str, requests_per_second: Option<u32>) -> Result<Self> {
        let rps = requests_per_second.unwrap_or(10);
        let rps_nz = NonZeroU32::new(rps).unwrap_or(NonZeroU32::new(10).unwrap());

        let quota = Quota::per_second(rps_nz);
        let limiter = Arc::new(RateLimiter::direct(quota));

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(user_agent)
                .map_err(|e| EdgarError::Other(format!("Invalid User-Agent: {e}")))?,
        );
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/json, text/html, application/xml, */*"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .gzip(true)
            .deflate(true)
            .build()?;

        Ok(Self { client, limiter })
    }

    /// Perform a rate-limited GET request, returning the response body as text.
    pub async fn get_text(&self, url: &str) -> Result<String> {
        self.limiter.until_ready().await;

        let response = self.client.get(url).send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            return Err(EdgarError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(EdgarError::Api {
                status: status.as_u16(),
                body,
            });
        }

        Ok(response.text().await?)
    }

    /// Perform a rate-limited GET request, returning the response body as bytes.
    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.limiter.until_ready().await;

        let response = self.client.get(url).send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            return Err(EdgarError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(EdgarError::Api {
                status: status.as_u16(),
                body,
            });
        }

        Ok(response.bytes().await?.to_vec())
    }

    /// Perform a rate-limited GET request, deserializing the JSON response.
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let text = self.get_text(url).await?;
        serde_json::from_str(&text).map_err(EdgarError::Json)
    }
}
