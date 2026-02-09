use std::time::Duration;

use clap::Args;

use crate::client::EdgarClient;
use crate::error::Result;
use crate::watcher::events::WatcherEvent;
use crate::watcher::filter::WatchFilter;
use crate::watcher::{self, WatcherConfig};

#[derive(Args)]
pub struct WatchArgs {
    /// Comma-separated list of tickers to watch.
    #[arg(long)]
    pub tickers: Option<String>,

    /// Comma-separated list of CIKs to watch.
    #[arg(long)]
    pub ciks: Option<String>,

    /// Comma-separated list of form types to watch.
    #[arg(long)]
    pub forms: Option<String>,

    /// Poll interval in seconds.
    #[arg(long, default_value = "60")]
    pub interval: u64,
}

pub async fn run(args: WatchArgs, client: &EdgarClient) -> Result<()> {
    let mut filter = WatchFilter::new();

    if let Some(ref tickers) = args.tickers {
        // Resolve tickers to CIKs
        let ticker_list: Vec<String> = tickers.split(',').map(|t| t.trim().to_string()).collect();
        let mut ciks = std::collections::HashSet::new();
        for ticker in &ticker_list {
            match client.resolve_ticker(ticker).await {
                Ok(cik) => {
                    ciks.insert(cik.as_u64().to_string());
                    println!("Watching {} (CIK {})", ticker, cik);
                }
                Err(e) => {
                    eprintln!("Warning: could not resolve ticker {}: {}", ticker, e);
                }
            }
        }
        filter.ciks = ciks;
    }

    if let Some(ref ciks) = args.ciks {
        filter.ciks.extend(ciks.split(',').map(|c| c.trim().to_string()));
    }

    if let Some(ref forms) = args.forms {
        filter = filter.with_form_types(forms.split(',').map(|f| f.trim().to_string()));
    }

    let config = WatcherConfig {
        poll_interval: Duration::from_secs(args.interval),
        filter,
        ..Default::default()
    };

    let mut handle = watcher::start_watcher(client.http.clone(), config);

    println!("Press Ctrl+C to stop watching...");

    loop {
        match handle.rx.recv().await {
            Ok(WatcherEvent::NewFiling(entry)) => {
                println!(
                    "[{}] {} — {} ({})",
                    entry.filing_date.as_deref().unwrap_or("?"),
                    entry.company_name.as_deref().unwrap_or("Unknown"),
                    entry.form_type.as_deref().unwrap_or("?"),
                    entry.title,
                );
            }
            Ok(WatcherEvent::Error(err)) => {
                eprintln!("Error: {err}");
            }
            Ok(WatcherEvent::Started) => {
                println!("Watcher started, polling every {}s...", args.interval);
            }
            Ok(WatcherEvent::Stopped) => {
                println!("Watcher stopped.");
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    Ok(())
}
