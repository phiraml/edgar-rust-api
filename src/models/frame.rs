use serde::{Deserialize, Serialize};

use crate::models::cik::Cik;

/// Response from /api/xbrl/frames/{taxonomy}/{tag}/{unit}/{period}.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameResponse {
    pub taxonomy: String,
    pub tag: String,
    pub ccp: String,
    pub uom: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub pts: Option<u32>,
    pub data: Vec<FrameEntry>,
}

/// A single company's data point within a frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameEntry {
    pub accn: String,
    pub cik: Cik,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub loc: Option<String>,
    pub end: String,
    pub val: Option<f64>,
}
