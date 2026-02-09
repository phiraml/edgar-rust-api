use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::models::cik::Cik;
use crate::models::filing_type::FilingType;

/// A single filing, row-oriented (flattened from the columnar EDGAR format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filing {
    pub accession_number: String,
    pub filing_type: FilingType,
    pub filing_date: NaiveDate,
    pub report_date: Option<NaiveDate>,
    pub acceptance_datetime: Option<String>,
    pub act: Option<String>,
    pub file_number: Option<String>,
    pub film_number: Option<String>,
    pub items: Option<String>,
    pub size: Option<u64>,
    pub is_xbrl: bool,
    pub is_inline_xbrl: bool,
    pub primary_document: Option<String>,
    pub primary_doc_description: Option<String>,
}

impl Filing {
    /// URL to the filing document on EDGAR Archives.
    pub fn document_url(&self, cik: Cik) -> Option<String> {
        self.primary_document.as_ref().map(|doc| {
            let accession_no_dashes = self.accession_number.replace('-', "");
            format!(
                "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
                cik.as_u64(),
                accession_no_dashes,
                doc
            )
        })
    }

    /// URL to the filing index page on EDGAR.
    pub fn index_url(&self, cik: Cik) -> String {
        let accession_no_dashes = self.accession_number.replace('-', "");
        format!(
            "https://www.sec.gov/Archives/edgar/data/{}/{}/",
            cik.as_u64(),
            accession_no_dashes
        )
    }
}
