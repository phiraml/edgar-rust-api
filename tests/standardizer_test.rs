use edgar_lib::models::company::Company;
use edgar_lib::models::company_facts::CompanyFactsResponse;
use edgar_lib::models::cik::Cik;
use edgar_lib::standardizer::engine::StandardizationEngine;
use edgar_lib::standardizer::output::StandardMetric;

fn sample_company() -> Company {
    Company {
        cik: Cik::from(320193),
        name: "Apple Inc.".to_string(),
        tickers: vec!["AAPL".to_string()],
        exchanges: vec!["Nasdaq".to_string()],
        sic: Some("3571".to_string()),
        sic_description: Some("Electronic Computers".to_string()),
        state_of_incorporation: Some("CA".to_string()),
        fiscal_year_end: Some("0930".to_string()),
        entity_type: None,
        category: None,
        ein: None,
        phone: None,
        website: None,
        investor_website: None,
        description: None,
    }
}

fn load_sample_facts() -> CompanyFactsResponse {
    let json = include_str!("fixtures/company_facts_sample.json");
    serde_json::from_str(json).expect("Failed to parse sample company facts")
}

#[test]
fn test_standardize_revenue() {
    let facts = load_sample_facts();
    let company = sample_company();
    let engine = StandardizationEngine::new();

    let financials = engine.standardize(&facts, &company).unwrap();

    assert_eq!(financials.entity_name, "Apple Inc.");
    assert_eq!(financials.cik, 320193);
    assert!(!financials.annual.is_empty(), "Should have annual periods");

    // Check FY2023 revenue
    let fy2023 = financials
        .annual
        .iter()
        .find(|p| p.period.year == 2023)
        .expect("Should have FY2023 data");

    let revenue = fy2023.get(&StandardMetric::Revenue).expect("Should have revenue");
    assert!(
        (revenue - 383_285_000_000.0).abs() < 1.0,
        "Revenue should be ~383B, got {}",
        revenue
    );
}

#[test]
fn test_standardize_net_income() {
    let facts = load_sample_facts();
    let company = sample_company();
    let engine = StandardizationEngine::new();

    let financials = engine.standardize(&facts, &company).unwrap();

    let fy2022 = financials
        .annual
        .iter()
        .find(|p| p.period.year == 2022)
        .expect("Should have FY2022 data");

    let net_income = fy2022.get(&StandardMetric::NetIncome).expect("Should have net income");
    assert!(
        (net_income - 99_803_000_000.0).abs() < 1.0,
        "Net income should be ~99.8B, got {}",
        net_income
    );
}

#[test]
fn test_standardize_eps() {
    let facts = load_sample_facts();
    let company = sample_company();
    let engine = StandardizationEngine::new();

    let financials = engine.standardize(&facts, &company).unwrap();

    let fy2023 = financials
        .annual
        .iter()
        .find(|p| p.period.year == 2023)
        .expect("Should have FY2023 data");

    let eps = fy2023
        .get(&StandardMetric::EarningsPerShareDiluted)
        .expect("Should have EPS");
    assert!(
        (eps - 6.16).abs() < 0.01,
        "EPS should be ~6.16, got {}",
        eps
    );
}

#[test]
fn test_standardize_ratios() {
    let facts = load_sample_facts();
    let company = sample_company();
    let engine = StandardizationEngine::new();

    let financials = engine.standardize(&facts, &company).unwrap();

    let fy2023 = financials
        .annual
        .iter()
        .find(|p| p.period.year == 2023)
        .expect("Should have FY2023 data");

    // ROE = Net Income / Stockholders' Equity
    let roe = fy2023.get(&StandardMetric::ReturnOnEquity).expect("Should have ROE");
    let expected_roe = 96_995_000_000.0 / 62_146_000_000.0;
    assert!(
        (roe - expected_roe).abs() < 0.01,
        "ROE should be ~{:.4}, got {:.4}",
        expected_roe,
        roe
    );
}

#[test]
fn test_multiple_periods() {
    let facts = load_sample_facts();
    let company = sample_company();
    let engine = StandardizationEngine::new();

    let financials = engine.standardize(&facts, &company).unwrap();

    // Should have 3 annual periods (2021, 2022, 2023)
    assert!(
        financials.annual.len() >= 3,
        "Should have at least 3 annual periods, got {}",
        financials.annual.len()
    );

    // Verify chronological ordering
    let years: Vec<i32> = financials.annual.iter().map(|p| p.period.year).collect();
    let mut sorted = years.clone();
    sorted.sort();
    assert_eq!(years, sorted, "Periods should be chronologically ordered");
}

#[test]
fn test_latest_annual() {
    let facts = load_sample_facts();
    let company = sample_company();
    let engine = StandardizationEngine::new();

    let financials = engine.standardize(&facts, &company).unwrap();

    let latest = financials.latest_annual().expect("Should have latest annual");
    assert_eq!(latest.period.year, 2023);
}
