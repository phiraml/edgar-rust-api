use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    tracing_subscriber::fmt::init();

    // Build client — optionally with Anthropic API key for LLM-enhanced analysis
    let mut builder = EdgarClient::builder("rohan.farmington@gmail.com")
        .tag_store_path("discovered_tags.json");
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        println!("Anthropic API key detected — LLM classification enabled\n");
        builder = builder.anthropic_api_key(api_key);
    } else {
        println!("No ANTHROPIC_API_KEY set — keyword-only analysis\n");
    }
    let client = builder.build()?;

    let tickers = &[
        // Tech
        "AAPL", "MSFT", "AMZN", "GOOGL", "META", "NVDA", "TSLA", "NFLX", "CRM", "ORCL",
        "INTC", "AMD", "ADBE", "CSCO", "IBM",
        // Finance
        "JPM", "BAC", "WFC", "GS", "MS", "BRK-B", "V", "MA", "AXP", "BLK",
        // Healthcare
        "JNJ", "UNH", "PFE", "ABBV", "MRK", "LLY", "TMO", "ABT",
        // Consumer
        "WMT", "PG", "KO", "PEP", "COST", "HD", "MCD", "NKE", "SBUX", "TGT",
        // Energy
        "XOM", "CVX", "COP", "SLB", "EOG",
        // Industrial
        "CAT", "BA", "UPS", "HON", "GE", "MMM", "LMT", "RTX",
        // Telecom / Media
        "DIS", "CMCSA", "T", "VZ", "TMUS",
    ];

    let total = tickers.len();
    for (i, ticker) in tickers.iter().enumerate() {
        println!("{}", "=".repeat(60));
        println!("  [{}/{}] Coverage Gap Analysis: {ticker}", i + 1, total);
        println!("{}\n", "=".repeat(60));

        let report = match client.coverage_gaps(ticker).await {
            Ok(r) => r,
            Err(e) => {
                println!("  ERROR: {e}\n\n");
                continue;
            }
        };

        println!(
            "Entity:   {}\nPeriod:   {}\nCoverage: {}/{} metrics ({:.1}%)\n",
            report.entity_name,
            report.period,
            report.resolved_count,
            report.expected_count,
            report.coverage_pct,
        );

        if report.missing_metrics.is_empty() {
            println!("  No missing metrics!\n");
        } else {
            println!("  Missing Metrics:");
            for mm in &report.missing_metrics {
                println!("    {} ({})", mm.display_name, mm.tags_tried.len());
                if mm.candidates.is_empty() {
                    println!("      No candidates found");
                } else {
                    for c in &mm.candidates {
                        let val_str = match c.latest_value {
                            Some(v) => format!("${:.0}M", v / 1_000_000.0),
                            None => "N/A".to_string(),
                        };
                        println!(
                            "      -> {}:{} | {} | {} | {}",
                            c.taxonomy,
                            c.tag,
                            c.label.as_deref().unwrap_or("(no label)"),
                            val_str,
                            c.match_reason,
                        );
                    }
                }
            }
            println!();
        }

        if !report.statement_gaps.is_empty() {
            println!("  Statement Gaps:");
            for gap in &report.statement_gaps {
                println!(
                    "    {:?}: total=${:.0}M, known=${:.0}M, unexplained=${:.0}M ({:.1}%)",
                    gap.total_metric,
                    gap.total_value / 1_000_000.0,
                    gap.known_sum / 1_000_000.0,
                    gap.unexplained_amount / 1_000_000.0,
                    gap.unexplained_pct,
                );
                for (m, v) in &gap.known_components {
                    println!("      - {:?}: ${:.0}M", m, v / 1_000_000.0);
                }
            }
            println!();
        }

        println!();
    }

    Ok(())
}
