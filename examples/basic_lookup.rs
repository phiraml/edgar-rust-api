use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("your-email@example.com")?;

    // Look up a company by ticker
    let company = client.company("AAPL").await?;
    println!("Company: {}", company.name);
    println!("CIK: {}", company.cik);
    println!("Tickers: {:?}", company.tickers);
    println!("SIC: {:?}", company.sic_description);

    // Get recent filings
    let filings = client.filings("AAPL").await?;
    println!("\nRecent filings:");
    for filing in filings.iter().take(5) {
        println!(
            "  {} | {} | {}",
            filing.filing_date,
            filing.filing_type,
            filing.primary_doc_description.as_deref().unwrap_or("")
        );
    }

    Ok(())
}
