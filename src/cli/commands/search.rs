use clap::Args;

use crate::cli::output::OutputFormat;
use crate::client::EdgarClient;
use crate::error::Result;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query.
    pub query: String,

    /// Filter by form type (e.g., "10-K").
    #[arg(long)]
    pub forms: Option<String>,

    /// Start date (YYYY-MM-DD).
    #[arg(long)]
    pub since: Option<String>,

    /// End date (YYYY-MM-DD).
    #[arg(long)]
    pub until: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}

pub async fn run(args: SearchArgs, client: &EdgarClient) -> Result<()> {
    let result = client
        .search(
            &args.query,
            args.forms.as_deref(),
            args.since.as_deref(),
            args.until.as_deref(),
            0,
        )
        .await?;

    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            println!(
                "Found {} results",
                result.hits.total.value
            );
            println!();

            for hit in &result.hits.hits {
                let src = &hit.source;
                let form = src.form_type.as_deref()
                    .or(src.form.as_deref())
                    .unwrap_or("?");
                let name = src.entity_name.as_deref()
                    .or_else(|| src.display_names.as_ref().and_then(|v| v.first().map(|s| s.as_str())))
                    .unwrap_or("?");
                println!(
                    "  {} | {} | {} | {}",
                    src.file_date.as_deref().unwrap_or("?"),
                    form,
                    name,
                    src.file_description.as_deref().unwrap_or(""),
                );
            }
        }
    }

    Ok(())
}
