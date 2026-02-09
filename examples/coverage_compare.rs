//! Compare coverage with and without the discovered tag store.
//!
//! Usage: cargo run --release --example coverage_compare

use edgar_lib::EdgarClient;

const TICKERS: &[&str] = &[
    "AAPL",   // Tech, clean filer
    "AMZN",   // Tech, known R&D gap
    "MSFT",   // Tech
    "JPM",    // Bank, sector-specific
    "BAC",    // Bank
    "BRK-B",  // Conglomerate
    "XOM",    // Energy
    "T",      // Telecom
    "WMT",    // Retail
    "PFE",    // Pharma
    "GE",     // Industrial conglomerate
    "DIS",    // Media
    "F",      // Auto
    "COIN",   // Crypto exchange
    "ABNB",   // Tech/travel
];

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    // Run WITHOUT tag store
    let client_bare = EdgarClient::builder("coverage-compare@example.com").build()?;

    // Run WITH tag store
    let client_tags = EdgarClient::builder("coverage-compare@example.com")
        .tag_store_path("discovered_tags.json")
        .build()?;

    println!(
        "{:<8} {:<35} {:>16} {:>16} {:>8}",
        "Ticker", "Entity", "Baseline", "With Tags", "New"
    );
    println!("{}", "-".repeat(85));

    for ticker in TICKERS {
        let name;
        let without;
        let with;

        match client_bare.coverage_gaps(ticker).await {
            Ok(r) => {
                name = r.entity_name.chars().take(33).collect::<String>();
                without = (r.resolved_count, r.expected_count, r.coverage_pct);
            }
            Err(e) => {
                println!("{:<8} ERROR: {e}", ticker);
                continue;
            }
        }

        match client_tags.coverage_gaps(ticker).await {
            Ok(r) => {
                with = (r.resolved_count, r.expected_count, r.coverage_pct);
            }
            Err(e) => {
                println!("{:<8} ERROR on tagged run: {e}", ticker);
                continue;
            }
        }

        let new_metrics = with.0 as i32 - without.0 as i32;
        let delta_str = if new_metrics > 0 {
            format!("+{}", new_metrics)
        } else {
            format!("{}", new_metrics)
        };

        println!(
            "{:<8} {:<35} {:>3}/{:<3} ({:>4.1}%)    {:>3}/{:<3} ({:>4.1}%)    {:>4}",
            ticker,
            name,
            without.0, without.1, without.2,
            with.0, with.1, with.2,
            delta_str,
        );
    }

    Ok(())
}
