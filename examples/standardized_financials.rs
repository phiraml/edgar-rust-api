use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let ticker = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "AAPL".to_string());

    let client = EdgarClient::builder("your-email@example.com")
        .tag_store_path("discovered_tags.json")
        .build()?;

    let financials = client.financials(&ticker).await?;

    println!("Financials for: {}", financials.entity_name);
    println!();

    if financials.annual.is_empty() {
        println!("No annual financial data available.");
    }

    for period in &financials.annual {
        println!("=== {} ===", period.period);
        for (metric, mv) in &period.metrics {
            if mv.value.abs() > 1_000_000.0 {
                println!("  {:?}: ${:.0}M", metric, mv.value / 1_000_000.0);
            } else {
                println!("  {:?}: {:.2}", metric, mv.value);
            }
        }
        println!();
    }

    Ok(())
}
