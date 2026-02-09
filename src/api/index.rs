use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::filing_index::IndexEntry;

const BASE: &str = "https://www.sec.gov/Archives/edgar/full-index";

/// Fetch the full-index company listing for a given year/quarter.
///
/// Returns parsed `IndexEntry` structs from the pipe-delimited company.idx file.
pub async fn fetch_full_index(
    http: &RateLimitedHttp,
    year: i32,
    quarter: u8,
) -> Result<Vec<IndexEntry>> {
    let url = format!("{}/{}/QTR{}/company.idx", BASE, year, quarter);
    let body = http.get_text(&url).await?;

    let entries = body
        .lines()
        .skip_while(|line| !line.contains("---")) // Skip header
        .skip(1) // Skip the dashes line
        .filter_map(IndexEntry::from_index_line)
        .collect();

    Ok(entries)
}
