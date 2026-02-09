use edgar_lib::standardizer::output::StandardMetric;
use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("rd-lookup@example.com")?;

    let tickers = [
        "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA", "NFLX",
        "CRM", "ADBE", "INTC", "AMD", "ORCL", "IBM", "CSCO",
    ];

    println!("{:<8} {:<30} {:>18} {:>18} {:>18}", "Ticker", "Company", "FY-2", "FY-1", "FY Latest");
    println!("{}", "-".repeat(94));

    for ticker in &tickers {
        match client.financials(ticker).await {
            Ok(fin) => {
                let annual = &fin.annual;
                let len = annual.len();
                let last3: Vec<_> = if len >= 3 {
                    annual[len - 3..].to_vec()
                } else {
                    annual.clone()
                };

                let vals: Vec<String> = last3
                    .iter()
                    .map(|p| {
                        match p.get(&StandardMetric::ResearchAndDevelopment) {
                            Some(v) => format!("${:.1}B", v / 1_000_000_000.0),
                            None => "—".to_string(),
                        }
                    })
                    .collect();

                // Pad to 3 columns
                let c0 = vals.get(0).cloned().unwrap_or_else(|| "—".to_string());
                let c1 = vals.get(1).cloned().unwrap_or_else(|| "—".to_string());
                let c2 = vals.get(2).cloned().unwrap_or_else(|| "—".to_string());

                println!("{:<8} {:<30} {:>18} {:>18} {:>18}", ticker, fin.entity_name, c0, c1, c2);
            }
            Err(e) => {
                eprintln!("{:<8} ERROR: {}", ticker, e);
            }
        }
    }

    Ok(())
}
