use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("tag-lookup@example.com")?;
    let facts = client.company_facts("AMZN").await?;

    let expense_keywords = [
        "Fulfillment", "Marketing", "GeneralAndAdmin", "SellingGeneral",
        "TechnologyAndContent", "Selling", "Operating", "CostOf",
        "Research", "Expense",
    ];

    println!("Amazon expense-related tags (FY2024 annual USD values):\n");

    if let Some(gaap) = facts.facts.get("us-gaap") {
        let mut hits: Vec<(&str, f64, &str)> = Vec::new();
        for (tag, data) in gaap {
            let tag_lower = tag.to_lowercase();
            let is_expense = expense_keywords.iter().any(|kw| tag_lower.contains(&kw.to_lowercase()));
            if !is_expense { continue; }

            if let Some(usd) = data.units.get("USD") {
                if let Some(v) = usd.iter()
                    .filter(|v| v.fiscal_year == Some(2024) && v.fiscal_period.as_deref() == Some("FY"))
                    .last()
                {
                    hits.push((tag, v.val.unwrap_or(0.0), data.label.as_deref().unwrap_or("")));
                }
            }
        }
        hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (tag, val, label) in &hits {
            println!("  {:<60} ${:>8.1}B   {}", tag, val / 1e9, label);
        }
    }

    Ok(())
}
