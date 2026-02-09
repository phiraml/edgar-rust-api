use quick_xml::de::from_str;
use serde::Deserialize;

use crate::client::http::RateLimitedHttp;
use crate::error::Result;
use crate::models::feed::FeedEntry;

const RSS_URL: &str = "https://www.sec.gov/cgi-bin/browse-edgar?action=getcurrent&type=&dateb=&owner=include&count=40&search_text=&start=0&output=atom";

/// Raw RSS/Atom feed from SEC.
#[derive(Debug, Deserialize)]
struct AtomFeed {
    #[serde(rename = "entry", default)]
    entries: Vec<AtomEntry>,
}

#[derive(Debug, Deserialize)]
struct AtomEntry {
    title: Option<String>,
    link: Option<AtomLink>,
    summary: Option<String>,
    updated: Option<String>,
    #[serde(rename = "accession-number")]
    accession_number: Option<String>,
    #[serde(rename = "cik")]
    cik: Option<String>,
    #[serde(rename = "form-type")]
    form_type: Option<String>,
    #[serde(rename = "filing-date")]
    filing_date: Option<String>,
    #[serde(rename = "company-name")]
    company_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AtomLink {
    #[serde(rename = "@href")]
    href: Option<String>,
}

/// Fetch recent filings from the SEC RSS/Atom feed.
pub async fn fetch_recent_feed(http: &RateLimitedHttp) -> Result<Vec<FeedEntry>> {
    let body = http.get_text(RSS_URL).await?;
    parse_feed(&body)
}

/// Fetch recent filings from a custom RSS URL.
pub async fn fetch_feed(http: &RateLimitedHttp, url: &str) -> Result<Vec<FeedEntry>> {
    let body = http.get_text(url).await?;
    parse_feed(&body)
}

fn parse_feed(xml: &str) -> Result<Vec<FeedEntry>> {
    let feed: AtomFeed = from_str(xml)?;
    let entries = feed
        .entries
        .into_iter()
        .map(|e| FeedEntry {
            title: e.title.unwrap_or_default(),
            link: e
                .link
                .and_then(|l| l.href)
                .unwrap_or_default(),
            description: e.summary,
            pub_date: e.updated,
            accession_number: e.accession_number,
            cik: e.cik,
            form_type: e.form_type,
            filing_date: e.filing_date,
            company_name: e.company_name,
        })
        .collect();
    Ok(entries)
}
