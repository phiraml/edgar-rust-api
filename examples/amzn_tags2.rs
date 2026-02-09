use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("tag-lookup@example.com")?;
    let facts = client.company_facts("AMZN").await?;

    // Look for large USD expenses that could be R&D
    // Amazon's "Technology and content" is ~$80B+, so look for big numbers
    println!("Amazon us-gaap USD tags with recent annual values > $10B:\n");

    if let Some(gaap) = facts.facts.get("us-gaap") {
        let mut hits: Vec<(&str, f64, &str)> = Vec::new();

        for (tag, data) in gaap {
            if let Some(usd_values) = data.units.get("USD") {
                for val in usd_values {
                    if val.fiscal_year == Some(2024)
                        && val.fiscal_period.as_deref() == Some("FY")
                        && val.val.unwrap_or(0.0).abs() > 10_000_000_000.0
                    {
                        hits.push((
                            tag,
                            val.val.unwrap_or(0.0),
                            data.label.as_deref().unwrap_or(""),
                        ));
                    }
                }
            }
        }

        hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (tag, val, label) in &hits {
            println!("  {:<65} ${:>10.1}B   {}", tag, val / 1e9, label);
        }
    }

    Ok(())
}
