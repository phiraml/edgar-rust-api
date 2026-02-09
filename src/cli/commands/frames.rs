use clap::Args;

use crate::cli::output::OutputFormat;
use crate::client::EdgarClient;
use crate::error::Result;
use crate::models::period::CalendarPeriod;

#[derive(Args)]
pub struct FramesArgs {
    /// XBRL taxonomy (e.g., "us-gaap").
    #[arg(default_value = "us-gaap")]
    pub taxonomy: String,

    /// XBRL tag (e.g., "Revenues").
    pub tag: String,

    /// Unit (e.g., "USD").
    #[arg(default_value = "USD")]
    pub unit: String,

    /// Calendar period (e.g., "CY2023", "CY2023Q1").
    pub period: String,

    /// Maximum entries to show.
    #[arg(long, default_value = "20")]
    pub limit: usize,

    /// Output format.
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn run(args: FramesArgs, client: &EdgarClient) -> Result<()> {
    let period: CalendarPeriod = args.period.parse()?;
    let frame = client
        .frame(&args.taxonomy, &args.tag, &args.unit, &period)
        .await?;

    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&frame)?);
        }
        _ => {
            println!(
                "Frame: {}/{}/{} for {}",
                frame.taxonomy, frame.tag, frame.uom, frame.ccp
            );
            if let Some(pts) = frame.pts {
                println!("Total data points: {pts}");
            }
            println!();

            let limit = args.limit.min(frame.data.len());
            for entry in &frame.data[..limit] {
                println!(
                    "  {} (CIK {}) = {:>15}  [{}]",
                    entry.entity_name,
                    entry.cik,
                    entry
                        .val
                        .map(|v| format!("{:.0}", v))
                        .unwrap_or_else(|| "N/A".to_string()),
                    entry.end,
                );
            }

            if frame.data.len() > limit {
                println!("  ... and {} more", frame.data.len() - limit);
            }
        }
    }

    Ok(())
}
