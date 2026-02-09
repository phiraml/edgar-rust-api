use std::collections::{HashMap, HashSet};
use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("tag-scan@example.com")?;

    let tickers = [
        "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA", "NFLX",
        "CRM", "ADBE", "INTC", "AMD", "ORCL", "IBM", "CSCO", "QCOM",
        "TXN", "AVGO", "NOW", "SNOW",
        "JNJ", "PFE", "LLY", "ABBV", "MRK", "BMY", "AMGN", "GILD",
        "GE", "MMM", "CAT", "HON", "BA", "LMT", "RTX",
        "F", "GM",
        "PG", "KO", "PEP", "WMT", "COST",
    ];

    // For each concept we standardize, find what tags companies actually use
    // by looking for tags that correlate with our standard concepts
    let concepts: Vec<(&str, Vec<&str>)> = vec![
        ("Revenue", vec!["revenue", "sales"]),
        ("Cost of Revenue", vec!["costof", "cogs"]),
        ("Gross Profit", vec!["grossprofit"]),
        ("R&D", vec!["research", "development", "technology"]),
        ("SG&A", vec!["sellinggeneral", "sellingand", "generalandadmin"]),
        ("Operating Income", vec!["operatingincome"]),
        ("Interest Expense", vec!["interestexpense"]),
        ("Net Income", vec!["netincome", "profitloss"]),
        ("Total Assets", vec!["assets"]),
        ("Total Liabilities", vec!["liabilities"]),
        ("Stockholders Equity", vec!["stockholdersequity", "shareholdersequity"]),
        ("Operating Cash Flow", vec!["netcashprovided", "operatingactivit"]),
        ("CapEx", vec!["paymentstoacquireproperty", "capitalexpenditure"]),
        ("Depreciation", vec!["depreciation"]),
        ("EPS", vec!["earningspershare"]),
        ("Shares Outstanding", vec!["sharesoutstanding", "weightedaveragenumber"]),
        ("Dividends", vec!["dividend"]),
        ("Long-term Debt", vec!["longtermdebt"]),
        ("Cash", vec!["cashandcash", "cashcash"]),
        ("Accounts Receivable", vec!["accountsreceivable", "receivablesnet"]),
        ("Inventory", vec!["inventorynet", "inventoryfinished"]),
        ("Accounts Payable", vec!["accountspayable"]),
        ("Share Repurchases", vec!["repurchaseofcommon", "repurchaseofequity"]),
    ];

    // tag -> (count, [tickers])
    let mut tag_usage: HashMap<String, (u32, HashSet<String>)> = HashMap::new();

    for ticker in &tickers {
        eprint!("{:<6}", ticker);
        match client.company_facts(ticker).await {
            Ok(facts) => {
                for (taxonomy, tags) in &facts.facts {
                    if taxonomy != "us-gaap" { continue; }
                    for (tag, data) in tags {
                        // Only count tags with USD FY values
                        let has_fy_usd = data.units.get("USD").map_or(false, |vals| {
                            vals.iter().any(|v| v.fiscal_period.as_deref() == Some("FY"))
                        }) || data.units.get("USD/shares").map_or(false, |vals| {
                            vals.iter().any(|v| v.fiscal_period.as_deref() == Some("FY"))
                        }) || data.units.get("shares").map_or(false, |vals| {
                            vals.iter().any(|v| v.fiscal_period.as_deref() == Some("FY"))
                        });

                        if has_fy_usd {
                            let entry = tag_usage.entry(tag.clone()).or_insert_with(|| (0, HashSet::new()));
                            entry.0 += 1;
                            entry.1.insert(ticker.to_string());
                        }
                    }
                }
                eprintln!(" OK");
            }
            Err(e) => eprintln!(" ERR: {}", e),
        }
    }

    // For each concept, find matching tags sorted by usage
    for (concept_name, keywords) in &concepts {
        println!("\n=== {} ===", concept_name);
        let mut matches: Vec<(&str, u32)> = tag_usage.iter()
            .filter(|(tag, _)| {
                let lower = tag.to_lowercase();
                keywords.iter().any(|kw| lower.contains(kw))
            })
            .map(|(tag, (count, _))| (tag.as_str(), *count))
            .collect();
        matches.sort_by(|a, b| b.1.cmp(&a.1));
        for (tag, count) in matches.iter().take(15) {
            println!("  {:<80} {}/42", tag, count);
        }
    }

    Ok(())
}
