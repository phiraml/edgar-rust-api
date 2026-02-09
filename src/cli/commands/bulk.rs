use clap::Args;

use crate::client::EdgarClient;
use crate::error::Result;
use crate::api;

#[derive(Args)]
pub struct BulkArgs {
    /// Type of bulk download: "facts" or "submissions".
    #[arg(value_enum)]
    pub data_type: BulkDataType,

    /// Output directory for extracted files.
    #[arg(long, default_value = ".")]
    pub output: String,

    /// Only count entries, don't extract.
    #[arg(long)]
    pub count_only: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum BulkDataType {
    Facts,
    Submissions,
}

pub async fn run(args: BulkArgs, client: &EdgarClient) -> Result<()> {
    println!("Downloading bulk data (this may take a while)...");

    let entries = match args.data_type {
        BulkDataType::Facts => {
            api::bulk::download_company_facts_bulk(&client.http).await?
        }
        BulkDataType::Submissions => {
            api::bulk::download_submissions_bulk(&client.http).await?
        }
    };

    println!("Extracted {} files.", entries.len());

    if args.count_only {
        return Ok(());
    }

    // Write files to output directory
    let output_dir = std::path::Path::new(&args.output);
    std::fs::create_dir_all(output_dir)?;

    for (name, content) in &entries {
        let path = output_dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
    }

    println!("Files written to {}", args.output);
    Ok(())
}
