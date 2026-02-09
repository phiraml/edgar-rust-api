use std::time::Duration;

use moka::future::Cache;

/// TTL presets for different EDGAR data types.
pub struct CacheTtl;

impl CacheTtl {
    pub const TICKER_MAP: Duration = Duration::from_secs(24 * 60 * 60); // 24h
    pub const COMPANY_FACTS: Duration = Duration::from_secs(15 * 60); // 15min
    pub const SUBMISSIONS: Duration = Duration::from_secs(5 * 60); // 5min
    pub const FRAMES: Duration = Duration::from_secs(60 * 60); // 1h
    pub const SEARCH: Duration = Duration::from_secs(2 * 60); // 2min
    pub const COMPANY_CONCEPT: Duration = Duration::from_secs(15 * 60); // 15min
}

/// LRU cache backed by moka.
///
/// Uses a global TTL set to the longest needed (24h for ticker map).
/// Individual entries naturally expire as they get evicted or when the
/// cache is accessed after their logical TTL (handled at the caller level
/// if needed). For simplicity, we use a single TTL for all entries.
#[derive(Clone)]
pub struct EdgarCache {
    inner: Cache<String, String>,
}

impl EdgarCache {
    pub fn new(max_capacity: u64) -> Self {
        let inner = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(24 * 60 * 60)) // Max TTL = 24h
            .build();

        Self { inner }
    }

    /// Get a cached value by key.
    pub async fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key).await
    }

    /// Insert a value. The `_ttl` parameter is for documentation purposes;
    /// eviction uses the global TTL set at construction.
    pub async fn insert(&self, key: String, value: String, _ttl: Duration) {
        self.inner.insert(key, value).await;
    }

    /// Invalidate a cached entry.
    pub async fn invalidate(&self, key: &str) {
        self.inner.invalidate(key).await;
    }

    /// Clear all cached entries.
    pub async fn clear(&self) {
        self.inner.invalidate_all();
    }
}
