use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use edgar_lib::EdgarClient;

const TAG_STORE_PATH: &str = "discovered_tags.json";
const PROGRESS_PATH: &str = "coverage_progress.csv";

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    tracing_subscriber::fmt::init();

    // Build client
    let mut builder = EdgarClient::builder("rohan.farmington@gmail.com")
        .tag_store_path(TAG_STORE_PATH);
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        println!("LLM classification enabled\n");
        builder = builder.anthropic_api_key(api_key);
    } else {
        println!("Keyword-only analysis (no ANTHROPIC_API_KEY)\n");
    }
    let client = builder.build()?;

    // Load ticker map and deduplicate by CIK (keep shortest ticker per company)
    println!("Loading ticker map from SEC...");
    let ticker_map = client.load_ticker_map().await?;
    println!("Raw ticker count: {}", ticker_map.len());

    // Deduplicate: for each CIK, keep the shortest ticker (primary symbol)
    let mut cik_to_ticker: std::collections::HashMap<u64, String> =
        std::collections::HashMap::new();
    for (ticker, entry) in &ticker_map.by_ticker {
        let cik = entry.cik_str.as_u64();
        let keep = match cik_to_ticker.get(&cik) {
            Some(existing) => ticker.len() < existing.len(),
            None => true,
        };
        if keep {
            cik_to_ticker.insert(cik, ticker.clone());
        }
    }

    let mut tickers: Vec<_> = cik_to_ticker.into_values().collect();
    tickers.sort();
    println!("Unique companies (by CIK): {}\n", tickers.len());

    // Load already-processed tickers (for resume)
    let done: HashSet<String> = load_done_tickers(PROGRESS_PATH);
    let remaining = tickers.iter().filter(|t| !done.contains(t.as_str())).count();
    if !done.is_empty() {
        println!("Resuming: {} already done, {} remaining\n", done.len(), remaining);
    }

    // Open CSV for append
    let csv_exists = Path::new(PROGRESS_PATH).exists();
    let mut csv = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(PROGRESS_PATH)
        .expect("Failed to open progress CSV");

    if !csv_exists {
        writeln!(csv, "ticker,entity_name,period,expected,resolved,coverage_pct,missing_count,gap_count")
            .expect("Failed to write CSV header");
    }

    let total = tickers.len();
    let mut processed = done.len();
    let mut success = 0u64;
    let mut errors = 0u64;

    for ticker in &tickers {
        if done.contains(ticker.as_str()) {
            continue;
        }

        processed += 1;
        print!("[{processed}/{total}] {ticker: <10}");
        std::io::stdout().flush().ok();

        match client.coverage_gaps(ticker).await {
            Ok(report) => {
                success += 1;
                println!(
                    " {} | {}/{} ({:.1}%) | {} missing | {} gaps",
                    report.entity_name,
                    report.resolved_count,
                    report.expected_count,
                    report.coverage_pct,
                    report.missing_metrics.len(),
                    report.statement_gaps.len(),
                );

                // Escape entity name for CSV
                let name = report.entity_name.replace('"', "\"\"");
                writeln!(
                    csv,
                    "{},\"{}\",{},{},{},{:.1},{},{}",
                    ticker,
                    name,
                    report.period,
                    report.expected_count,
                    report.resolved_count,
                    report.coverage_pct,
                    report.missing_metrics.len(),
                    report.statement_gaps.len(),
                )
                .ok();
                csv.flush().ok();
            }
            Err(e) => {
                errors += 1;
                let err_msg = format!("{e}");
                let short = if err_msg.len() > 80 {
                    format!("{}...", &err_msg[..80])
                } else {
                    err_msg.clone()
                };
                println!(" ERROR: {short}");

                writeln!(csv, "{},\"ERROR: {}\",,,,,", ticker, err_msg.replace('"', "\"\"")).ok();
                csv.flush().ok();
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("  COMPLETE");
    println!("  Total: {total}  Success: {success}  Errors: {errors}");
    println!("  Results: {PROGRESS_PATH}");
    println!("  Tags:    {TAG_STORE_PATH}");
    println!("{}", "=".repeat(60));

    Ok(())
}

fn load_done_tickers(path: &str) -> HashSet<String> {
    let mut done = HashSet::new();
    if let Ok(data) = std::fs::read_to_string(path) {
        for line in data.lines().skip(1) {
            // First column is ticker
            if let Some(ticker) = line.split(',').next() {
                if !ticker.is_empty() {
                    done.insert(ticker.to_string());
                }
            }
        }
    }
    done
}
