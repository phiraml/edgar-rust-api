use clap::Args;

use crate::cli::output::{self, OutputFormat};
use crate::client::EdgarClient;
use crate::error::Result;

#[derive(Args)]
pub struct FinancialsArgs {
    /// Ticker symbol or CIK number.
    pub identifier: String,

    /// Include computed ratios.
    #[arg(long)]
    pub ratios: bool,

    /// Show quarterly data instead of annual.
    #[arg(long)]
    pub quarterly: bool,

    /// Output format.
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn run(args: FinancialsArgs, client: &EdgarClient) -> Result<()> {
    let financials = client.financials(&args.identifier).await?;
    output::print_financials(&financials, args.format, args.ratios, args.quarterly)?;
    Ok(())
}
