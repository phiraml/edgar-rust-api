use std::collections::HashMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::models::cik::Cik;

/// Top-level response from /api/xbrl/companyfacts/CIK{cik}.json
#[derive(Debug, Clone, Deserialize)]
pub struct CompanyFactsResponse {
    pub cik: Cik,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    /// Map of taxonomy (e.g., "us-gaap", "dei") to tag data.
    pub facts: HashMap<String, HashMap<String, TagData>>,
}

/// Data associated with a single XBRL tag.
#[derive(Debug, Clone, Deserialize)]
pub struct TagData {
    pub label: Option<String>,
    pub description: Option<String>,
    /// Map of unit type (e.g., "USD", "shares") to fact values.
    pub units: HashMap<String, Vec<FactValue>>,
}

/// A single fact data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactValue {
    /// The date the filing was submitted.
    #[serde(rename = "filed")]
    pub filed: String,
    /// The reporting period start date (for duration facts).
    pub start: Option<String>,
    /// The reporting period end date, or instant date.
    pub end: String,
    /// The numeric value.
    pub val: Option<f64>,
    /// Accession number of the filing.
    #[serde(rename = "accn")]
    pub accession: String,
    /// The form type (10-K, 10-Q, etc.)
    pub form: Option<String>,
    /// The fiscal year.
    #[serde(rename = "fy")]
    pub fiscal_year: Option<i32>,
    /// The fiscal period (FY, Q1, Q2, Q3, Q4).
    #[serde(rename = "fp")]
    pub fiscal_period: Option<String>,
    /// The SEC-assigned frame identifier (e.g., "CY2023Q1").
    pub frame: Option<String>,
}

impl FactValue {
    pub fn end_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.end, "%Y-%m-%d").ok()
    }

    pub fn start_date(&self) -> Option<NaiveDate> {
        self.start
            .as_ref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    }

    pub fn filed_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.filed, "%Y-%m-%d").ok()
    }

    /// True if this fact represents an annual period.
    pub fn is_annual(&self) -> bool {
        self.fiscal_period.as_deref() == Some("FY")
            || self.form.as_deref() == Some("10-K")
            || self.form.as_deref() == Some("20-F")
    }

    /// True if this fact represents a quarterly period.
    pub fn is_quarterly(&self) -> bool {
        matches!(
            self.fiscal_period.as_deref(),
            Some("Q1") | Some("Q2") | Some("Q3") | Some("Q4")
        )
    }

    /// Duration in days. Returns None for instant facts.
    pub fn duration_days(&self) -> Option<i64> {
        let start = self.start_date()?;
        let end = self.end_date()?;
        Some((end - start).num_days())
    }
}
