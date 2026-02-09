pub mod commands;
pub mod output;

use clap::{Parser, Subcommand};

use crate::client::EdgarClient;
use crate::error::Result;

/// SEC EDGAR CLI — query filings, financials, and more.
#[derive(Parser)]
#[command(name = "edgar", version, about)]
pub struct Cli {
    /// Your email address for the SEC User-Agent header.
    #[arg(long, env = "EDGAR_USER_AGENT", default_value = "edgar-lib/0.1.0")]
    pub user_agent: String,

    /// Path to a discovered tags JSON file for enhanced XBRL coverage.
    /// Defaults to "discovered_tags.json" if that file exists.
    #[arg(long, env = "EDGAR_TAG_STORE")]
    pub tag_store: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Look up company information.
    Company(commands::company::CompanyArgs),
    /// Full-text search over EDGAR filings.
    Search(commands::search::SearchArgs),
    /// Fetch cross-company XBRL frame data.
    Frames(commands::frames::FramesArgs),
    /// Get standardized financial statements.
    Financials(commands::financials::FinancialsArgs),
    /// Watch for new filings in real time.
    Watch(commands::watch::WatchArgs),
    /// Download bulk EDGAR data.
    Bulk(commands::bulk::BulkArgs),
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut builder = EdgarClient::builder(&cli.user_agent);
    let tag_path = cli
        .tag_store
        .unwrap_or_else(|| "discovered_tags.json".to_string());
    if std::path::Path::new(&tag_path).exists() {
        builder = builder.tag_store_path(tag_path);
    }
    let client = builder.build()?;

    match cli.command {
        Commands::Company(args) => commands::company::run(args, &client).await,
        Commands::Search(args) => commands::search::run(args, &client).await,
        Commands::Frames(args) => commands::frames::run(args, &client).await,
        Commands::Financials(args) => commands::financials::run(args, &client).await,
        Commands::Watch(args) => commands::watch::run(args, &client).await,
        Commands::Bulk(args) => commands::bulk::run(args, &client).await,
    }
}
