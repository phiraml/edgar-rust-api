use serde::{Deserialize, Serialize};

/// EDGAR full-text search response from /LATEST/search-index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: Option<SearchQuery>,
    pub hits: SearchHits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    #[serde(rename = "dateRange")]
    pub date_range: Option<DateRange>,
    #[serde(rename = "startdt")]
    pub start_date: Option<String>,
    #[serde(rename = "enddt")]
    pub end_date: Option<String>,
    pub forms: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    #[serde(rename = "startdt")]
    pub start: Option<String>,
    #[serde(rename = "enddt")]
    pub end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHits {
    pub total: SearchTotal,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTotal {
    pub value: u64,
    pub relation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    #[serde(rename = "_source")]
    pub source: SearchHitSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHitSource {
    #[serde(rename = "entity_name")]
    pub entity_name: Option<String>,
    #[serde(rename = "file_num")]
    pub file_num: Option<String>,
    #[serde(rename = "file_date")]
    pub file_date: Option<String>,
    pub period_of_report: Option<String>,
    pub form_type: Option<String>,
    pub file_description: Option<String>,
    pub display_names: Option<Vec<String>>,
    pub display_date_filed: Option<String>,
    pub entity_id: Option<String>,
    pub biz_locations: Option<Vec<String>>,
    pub inc_states: Option<Vec<String>>,
}
