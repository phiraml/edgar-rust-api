use serde::{Deserialize, Serialize};

/// A single entry from EDGAR full-index files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub cik: String,
    pub company_name: String,
    pub form_type: String,
    pub date_filed: String,
    pub filename: String,
}

impl IndexEntry {
    /// Parse a pipe-delimited line from EDGAR full-index.
    pub fn from_index_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 5 {
            return None;
        }
        Some(Self {
            cik: parts[0].trim().to_string(),
            company_name: parts[1].trim().to_string(),
            form_type: parts[2].trim().to_string(),
            date_filed: parts[3].trim().to_string(),
            filename: parts[4].trim().to_string(),
        })
    }

    pub fn filing_url(&self) -> String {
        format!("https://www.sec.gov/Archives/{}", self.filename)
    }
}
