use thiserror::Error;

/// Central error type for the edgar-lib crate.
#[derive(Debug, Error)]
pub enum EdgarError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("XML parsing failed: {0}")]
    Xml(#[from] quick_xml::DeError),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("ZIP extraction error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid CIK: {0}")]
    InvalidCik(String),

    #[error("Ticker not found: {0}")]
    TickerNotFound(String),

    #[error("Rate limited — retry after {retry_after_secs:?}s")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("EDGAR API returned {status}: {body}")]
    Api { status: u16, body: String },

    #[error("No data available for {metric} in period {period}")]
    NoData { metric: String, period: String },

    #[error("Invalid period format: {0}")]
    InvalidPeriod(String),

    #[error("Watcher error: {0}")]
    Watcher(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EdgarError>;
