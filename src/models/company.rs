use serde::{Deserialize, Serialize};

use crate::models::cik::Cik;

/// Company metadata from EDGAR submissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub cik: Cik,
    pub name: String,
    pub tickers: Vec<String>,
    pub exchanges: Vec<String>,
    pub sic: Option<String>,
    pub sic_description: Option<String>,
    pub state_of_incorporation: Option<String>,
    pub fiscal_year_end: Option<String>,
    pub entity_type: Option<String>,
    pub category: Option<String>,
    pub ein: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub investor_website: Option<String>,
    pub description: Option<String>,
}
