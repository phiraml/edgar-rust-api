use serde::{Deserialize, Serialize};

/// An entry from the SEC RSS feed for recent filings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEntry {
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub pub_date: Option<String>,
    pub accession_number: Option<String>,
    pub cik: Option<String>,
    pub form_type: Option<String>,
    pub filing_date: Option<String>,
    pub company_name: Option<String>,
}
