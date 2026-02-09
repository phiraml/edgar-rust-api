use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::cik::Cik;

/// A single entry from the SEC company_tickers.json file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyTicker {
    pub cik_str: Cik,
    pub ticker: String,
    pub title: String,
}

/// Maps tickers to CIKs and vice versa.
#[derive(Debug, Clone)]
pub struct TickerMap {
    pub by_ticker: HashMap<String, CompanyTicker>,
    pub by_cik: HashMap<u64, CompanyTicker>,
}

impl TickerMap {
    pub fn from_entries(entries: Vec<CompanyTicker>) -> Self {
        let mut by_ticker = HashMap::with_capacity(entries.len());
        let mut by_cik = HashMap::with_capacity(entries.len());

        for entry in entries {
            by_ticker.insert(entry.ticker.to_uppercase(), entry.clone());
            by_cik.insert(entry.cik_str.as_u64(), entry);
        }

        Self { by_ticker, by_cik }
    }

    pub fn lookup_ticker(&self, ticker: &str) -> Option<&CompanyTicker> {
        self.by_ticker.get(&ticker.to_uppercase())
    }

    pub fn lookup_cik(&self, cik: Cik) -> Option<&CompanyTicker> {
        self.by_cik.get(&cik.as_u64())
    }

    pub fn len(&self) -> usize {
        self.by_ticker.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_ticker.is_empty()
    }
}
