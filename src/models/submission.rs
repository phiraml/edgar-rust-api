use chrono::NaiveDate;
use serde::Deserialize;

use crate::models::cik::Cik;
use crate::models::company::Company;
use crate::models::filing::Filing;
use crate::models::filing_type::FilingType;

/// Raw EDGAR submissions API response.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmissionsResponse {
    pub cik: Cik,
    #[serde(rename = "entityType")]
    pub entity_type: Option<String>,
    pub sic: Option<String>,
    #[serde(rename = "sicDescription")]
    pub sic_description: Option<String>,
    pub name: Option<String>,
    pub tickers: Option<Vec<String>>,
    pub exchanges: Option<Vec<String>>,
    pub ein: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    #[serde(rename = "investorWebsite")]
    pub investor_website: Option<String>,
    pub category: Option<String>,
    #[serde(rename = "fiscalYearEnd")]
    pub fiscal_year_end: Option<String>,
    #[serde(rename = "stateOfIncorporation")]
    pub state_of_incorporation: Option<String>,
    pub phone: Option<String>,
    pub filings: FilingsContainer,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilingsContainer {
    pub recent: RecentFilings,
    pub files: Vec<FilingsFile>,
}

/// A paginated file reference for older filings.
#[derive(Debug, Clone, Deserialize)]
pub struct FilingsFile {
    pub name: String,
    #[serde(rename = "filingCount")]
    pub filing_count: u32,
    #[serde(rename = "filingFrom")]
    pub filing_from: Option<String>,
    #[serde(rename = "filingTo")]
    pub filing_to: Option<String>,
}

/// Columnar representation of recent filings from EDGAR.
#[derive(Debug, Clone, Deserialize)]
pub struct RecentFilings {
    #[serde(rename = "accessionNumber")]
    pub accession_number: Vec<String>,
    #[serde(rename = "filingDate")]
    pub filing_date: Vec<String>,
    #[serde(rename = "reportDate")]
    pub report_date: Vec<String>,
    #[serde(rename = "acceptanceDateTime")]
    pub acceptance_date_time: Vec<String>,
    pub act: Vec<String>,
    pub form: Vec<String>,
    #[serde(rename = "fileNumber")]
    pub file_number: Vec<String>,
    #[serde(rename = "filmNumber")]
    pub film_number: Vec<String>,
    pub items: Vec<String>,
    pub size: Vec<u64>,
    #[serde(rename = "isXBRL")]
    pub is_xbrl: Vec<u8>,
    #[serde(rename = "isInlineXBRL")]
    pub is_inline_xbrl: Vec<u8>,
    #[serde(rename = "primaryDocument")]
    pub primary_document: Vec<String>,
    #[serde(rename = "primaryDocDescription")]
    pub primary_doc_description: Vec<String>,
}

impl RecentFilings {
    /// Convert columnar data into row-oriented Filing structs.
    pub fn to_filings(&self) -> Vec<Filing> {
        let len = self.accession_number.len();
        let mut filings = Vec::with_capacity(len);

        for i in 0..len {
            let filing_date = NaiveDate::parse_from_str(&self.filing_date[i], "%Y-%m-%d")
                .unwrap_or_default();
            let report_date = NaiveDate::parse_from_str(&self.report_date[i], "%Y-%m-%d").ok();

            filings.push(Filing {
                accession_number: self.accession_number[i].clone(),
                filing_type: FilingType::parse_lenient(&self.form[i]),
                filing_date,
                report_date,
                acceptance_datetime: non_empty(&self.acceptance_date_time[i]),
                act: non_empty(&self.act[i]),
                file_number: non_empty(&self.file_number[i]),
                film_number: non_empty(&self.film_number[i]),
                items: non_empty(&self.items[i]),
                size: Some(self.size[i]),
                is_xbrl: self.is_xbrl[i] != 0,
                is_inline_xbrl: self.is_inline_xbrl[i] != 0,
                primary_document: non_empty(&self.primary_document[i]),
                primary_doc_description: non_empty(&self.primary_doc_description[i]),
            });
        }

        filings
    }
}

impl SubmissionsResponse {
    pub fn to_company(&self) -> Company {
        Company {
            cik: self.cik,
            name: self.name.clone().unwrap_or_default(),
            tickers: self.tickers.clone().unwrap_or_default(),
            exchanges: self.exchanges.clone().unwrap_or_default(),
            sic: self.sic.clone(),
            sic_description: self.sic_description.clone(),
            state_of_incorporation: self.state_of_incorporation.clone(),
            fiscal_year_end: self.fiscal_year_end.clone(),
            entity_type: self.entity_type.clone(),
            category: self.category.clone(),
            ein: self.ein.clone(),
            phone: self.phone.clone(),
            website: self.website.clone(),
            investor_website: self.investor_website.clone(),
            description: self.description.clone(),
        }
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
