use clap::Args;

use crate::cli::output::{self, OutputFormat};
use crate::client::EdgarClient;
use crate::error::Result;

#[derive(Args)]
pub struct CompanyArgs {
    /// Ticker symbol or CIK number.
    pub identifier: String,

    /// Also show recent filings.
    #[arg(long)]
    pub filings: bool,

    /// Number of filings to show (default: 20).
    #[arg(long, default_value = "20")]
    pub limit: usize,

    /// Output format.
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn run(args: CompanyArgs, client: &EdgarClient) -> Result<()> {
    let company = client.company(&args.identifier).await?;
    output::print_company(&company, args.format)?;

    if args.filings {
        println!();
        let filings = client.filings(&args.identifier).await?;
        output::print_filings(&filings, args.format, args.limit)?;
    }

    Ok(())
}
