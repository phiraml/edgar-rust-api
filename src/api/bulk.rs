use std::io::{Cursor, Read};

use zip::ZipArchive;

use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::company_facts::CompanyFactsResponse;
use crate::models::submission::SubmissionsResponse;

const COMPANYFACTS_ZIP: &str = "https://www.sec.gov/Archives/edgar/daily-index/xbrl/companyfacts.zip";
const SUBMISSIONS_ZIP: &str = "https://www.sec.gov/Archives/edgar/daily-index/bulk-data/submissions.zip";

/// Download and extract the companyfacts.zip bulk file.
///
/// Returns an iterator of (filename, raw JSON string) pairs.
pub async fn download_company_facts_bulk(
    http: &RateLimitedHttp,
) -> Result<Vec<(String, String)>> {
    let bytes = http.get_bytes(COMPANYFACTS_ZIP).await?;
    extract_zip_entries(bytes)
}

/// Download and extract the submissions.zip bulk file.
///
/// Returns an iterator of (filename, raw JSON string) pairs.
pub async fn download_submissions_bulk(
    http: &RateLimitedHttp,
) -> Result<Vec<(String, String)>> {
    let bytes = http.get_bytes(SUBMISSIONS_ZIP).await?;
    extract_zip_entries(bytes)
}

/// Parse a single company facts JSON string from bulk data.
pub fn parse_company_facts(json: &str) -> Result<CompanyFactsResponse> {
    Ok(serde_json::from_str(json)?)
}

/// Parse a single submissions JSON string from bulk data.
pub fn parse_submission(json: &str) -> Result<SubmissionsResponse> {
    Ok(serde_json::from_str(json)?)
}

fn extract_zip_entries(bytes: Vec<u8>) -> Result<Vec<(String, String)>> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut entries = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if name.ends_with(".json") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            entries.push((name, contents));
        }
    }

    Ok(entries)
}
