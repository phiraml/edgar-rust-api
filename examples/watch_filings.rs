use std::time::Duration;

use edgar_lib::watcher::events::WatcherEvent;
use edgar_lib::watcher::filter::WatchFilter;
use edgar_lib::watcher::WatcherConfig;
use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("your-email@example.com")?;

    // Watch for 10-K and 10-Q filings
    let filter = WatchFilter::new()
        .with_form_types(["10-K".to_string(), "10-Q".to_string()]);

    let config = WatcherConfig {
        poll_interval: Duration::from_secs(60),
        filter,
        ..Default::default()
    };

    let mut handle = client.start_watcher(config);

    println!("Watching for new 10-K and 10-Q filings...");
    println!("Press Ctrl+C to stop.");

    loop {
        match handle.rx.recv().await {
            Ok(WatcherEvent::NewFiling(entry)) => {
                println!(
                    "[NEW] {} — {} | {}",
                    entry.company_name.as_deref().unwrap_or("?"),
                    entry.form_type.as_deref().unwrap_or("?"),
                    entry.title,
                );
            }
            Ok(WatcherEvent::Error(err)) => eprintln!("Error: {err}"),
            Ok(WatcherEvent::Stopped) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }

    Ok(())
}
