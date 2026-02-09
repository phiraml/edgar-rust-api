use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("tag-lookup@example.com")?;
    let facts = client.company_facts("AMZN").await?;

    // List ALL taxonomies and look for non-us-gaap ones
    println!("Taxonomies present in Amazon's facts:");
    for (taxonomy, tags) in &facts.facts {
        println!("  {} ({} tags)", taxonomy, tags.len());
    }

    // Search ALL taxonomies for anything with "technology" or "content"
    println!("\nTags containing 'technolog' across all taxonomies:");
    for (taxonomy, tags) in &facts.facts {
        for (tag, data) in tags {
            if tag.to_lowercase().contains("technolog") {
                for (unit, values) in &data.units {
                    if let Some(v) = values.iter().filter(|v| v.fiscal_period.as_deref() == Some("FY")).last() {
                        println!(
                            "  {}/{}  [{}]  = {:.1}B  (fy={:?})",
                            taxonomy, tag, unit,
                            v.val.unwrap_or(0.0) / 1e9,
                            v.fiscal_year,
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
