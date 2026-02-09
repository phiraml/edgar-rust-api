use std::collections::HashMap;

use serde::Deserialize;

use crate::models::cik::Cik;
use crate::models::company_facts::FactValue;

/// Response from /api/xbrl/companyconcept/CIK{cik}/{taxonomy}/{tag}.json
#[derive(Debug, Clone, Deserialize)]
pub struct CompanyConceptResponse {
    pub cik: Cik,
    pub taxonomy: String,
    pub tag: String,
    pub label: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    /// Map of unit type (e.g., "USD", "shares") to fact values.
    pub units: HashMap<String, Vec<FactValue>>,
}
