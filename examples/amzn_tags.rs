use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("tag-lookup@example.com")?;
    let facts = client.company_facts("AMZN").await?;

    // Search for tags that might be R&D-related
    let keywords = ["research", "development", "technology", "content"];

    println!("Amazon XBRL tags matching R&D-related keywords:\n");

    for (taxonomy, tags) in &facts.facts {
        for (tag, data) in tags {
            let tag_lower = tag.to_lowercase();
            if keywords.iter().any(|kw| tag_lower.contains(kw)) {
                // Show tag with latest value
                for (unit, values) in &data.units {
                    if let Some(latest) = values.last() {
                        println!(
                            "  {}/{}  [{}]  = {:>15}  (fy={:?} fp={:?} form={:?})",
                            taxonomy,
                            tag,
                            unit,
                            latest.val.map(|v| format!("{:.0}", v)).unwrap_or_default(),
                            latest.fiscal_year,
                            latest.fiscal_period,
                            latest.form,
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
