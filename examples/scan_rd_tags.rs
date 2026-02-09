use std::collections::HashMap;
use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("tag-scan@example.com")?;

    // Broad set of companies across sectors
    let tickers = [
        // Tech (various sizes)
        "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA", "NFLX",
        "CRM", "ADBE", "INTC", "AMD", "ORCL", "IBM", "CSCO", "QCOM",
        "TXN", "AVGO", "NOW", "SNOW",
        // Pharma/Biotech
        "JNJ", "PFE", "LLY", "ABBV", "MRK", "BMY", "AMGN", "GILD",
        // Industrial / diversified
        "GE", "MMM", "CAT", "HON", "BA", "LMT", "RTX",
        // Auto
        "F", "GM",
        // Consumer
        "PG", "KO", "PEP", "WMT", "COST",
    ];

    // Collect: for each company, what taxonomies exist and what tags look like R&D
    let rd_keywords = [
        "research", "development", "rnd", "r&d", "technolog",
    ];

    let mut all_taxonomies: HashMap<String, u32> = HashMap::new();
    let mut rd_tag_hits: HashMap<String, Vec<String>> = HashMap::new(); // tag -> [tickers]
    let mut missing_rd: Vec<String> = Vec::new();

    for ticker in &tickers {
        eprint!("{:<6}", ticker);
        match client.company_facts(ticker).await {
            Ok(facts) => {
                let mut found_rd = false;

                for (taxonomy, tags) in &facts.facts {
                    *all_taxonomies.entry(taxonomy.clone()).or_insert(0) += 1;

                    for (tag, data) in tags {
                        let tag_lower = tag.to_lowercase();
                        let is_rd = rd_keywords.iter().any(|kw| tag_lower.contains(kw));

                        if is_rd {
                            // Check if it has USD values with FY data
                            let has_fy_usd = data.units.get("USD").map_or(false, |vals| {
                                vals.iter().any(|v| {
                                    v.fiscal_period.as_deref() == Some("FY")
                                        && v.val.unwrap_or(0.0).abs() > 1_000_000.0
                                })
                            });

                            if has_fy_usd {
                                let key = format!("{}/{}", taxonomy, tag);
                                rd_tag_hits.entry(key).or_default().push(ticker.to_string());
                                found_rd = true;
                            }
                        }
                    }
                }

                if !found_rd {
                    missing_rd.push(ticker.to_string());
                }
                eprintln!(" OK ({} taxonomies)", facts.facts.len());
            }
            Err(e) => {
                eprintln!(" ERR: {}", e);
            }
        }
    }

    println!("\n\n=== TAXONOMIES FOUND ===");
    let mut tax: Vec<_> = all_taxonomies.iter().collect();
    tax.sort_by(|a, b| b.1.cmp(a.1));
    for (t, count) in &tax {
        println!("  {:<20} used by {} companies", t, count);
    }

    println!("\n=== R&D-RELATED TAGS (with USD FY values) ===");
    let mut hits: Vec<_> = rd_tag_hits.iter().collect();
    hits.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    for (tag, tickers) in &hits {
        println!("  {:<75} ({} cos) {}", tag, tickers.len(),
            if tickers.len() <= 8 { tickers.join(", ") } else { format!("{}...", tickers[..8].join(", ")) });
    }

    println!("\n=== COMPANIES WITH NO R&D TAG ===");
    for t in &missing_rd {
        println!("  {}", t);
    }

    Ok(())
}
