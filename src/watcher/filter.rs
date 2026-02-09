use std::collections::HashSet;

use crate::models::feed::FeedEntry;
/// Filter configuration for the watcher.
#[derive(Debug, Clone, Default)]
pub struct WatchFilter {
    /// Only emit events for these CIKs (empty = all).
    pub ciks: HashSet<String>,
    /// Only emit events for these tickers (empty = all).
    pub tickers: HashSet<String>,
    /// Only emit events for these form types (empty = all).
    pub form_types: HashSet<String>,
}

impl WatchFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ciks(mut self, ciks: impl IntoIterator<Item = String>) -> Self {
        self.ciks = ciks.into_iter().collect();
        self
    }

    pub fn with_tickers(mut self, tickers: impl IntoIterator<Item = String>) -> Self {
        self.tickers = tickers.into_iter().map(|t| t.to_uppercase()).collect();
        self
    }

    pub fn with_form_types(mut self, types: impl IntoIterator<Item = String>) -> Self {
        self.form_types = types.into_iter().collect();
        self
    }

    /// Check if a feed entry passes this filter.
    pub fn matches(&self, entry: &FeedEntry) -> bool {
        // CIK filter
        if !self.ciks.is_empty() {
            if let Some(ref cik) = entry.cik {
                if !self.ciks.contains(cik) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Form type filter
        if !self.form_types.is_empty() {
            if let Some(ref form) = entry.form_type {
                if !self.form_types.contains(form) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Ticker filtering would need a ticker map lookup, so we handle it at a higher level
        true
    }
}
