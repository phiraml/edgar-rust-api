use edgar_lib::models::company::Company;
use edgar_lib::models::company_facts::CompanyFactsResponse;
use edgar_lib::models::cik::Cik;
use edgar_lib::standardizer::engine::StandardizationEngine;
use edgar_lib::models::period::{FiscalPeriod, Quarter};
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

// ── Period alignment regressions ────────────────────────────────────────
//
// EDGAR's `fy` is the fiscal year of the *filing*: a FY2025 10-K tags its
// FY2023/FY2024 comparatives `fy: 2025` too. Periods must come from dates.

fn company_with_fye(fye: &str) -> Company {
    Company {
        fiscal_year_end: Some(fye.to_string()),
        ..sample_company()
    }
}

fn fact(start: Option<&str>, end: &str, val: f64, fy: i32, fp: &str, form: &str, filed: &str) -> serde_json::Value {
    let mut f = serde_json::json!({
        "end": end, "val": val, "accn": format!("acc-{filed}"), "fy": fy, "fp": fp,
        "form": form, "filed": filed,
    });
    if let Some(s) = start {
        f["start"] = serde_json::json!(s);
    }
    f
}

fn facts_with_revenue(values: Vec<serde_json::Value>) -> CompanyFactsResponse {
    serde_json::from_value(serde_json::json!({
        "cik": 789019,
        "entityName": "Test Co",
        "facts": { "us-gaap": { "RevenueFromContractWithCustomerExcludingAssessedTax": {
            "label": "Revenue", "description": null, "units": { "USD": values }
        }}}
    }))
    .unwrap()
}

#[test]
fn test_10k_comparatives_land_in_their_own_fiscal_year() {
    // June FYE. The FY2025 10-K carries three years, all tagged fy=2025.
    let facts = facts_with_revenue(vec![
        fact(Some("2022-07-01"), "2023-06-30", 211.9e9, 2025, "FY", "10-K", "2025-07-30"),
        fact(Some("2023-07-01"), "2024-06-30", 245.1e9, 2025, "FY", "10-K", "2025-07-30"),
        fact(Some("2024-07-01"), "2025-06-30", 281.7e9, 2025, "FY", "10-K", "2025-07-30"),
    ]);
    let fin = StandardizationEngine::new()
        .standardize(&facts, &company_with_fye("0630"))
        .unwrap();

    let years: Vec<i32> = fin.annual.iter().map(|p| p.period.year).collect();
    assert_eq!(years, vec![2023, 2024, 2025]);
    let rev = |y: i32| {
        fin.annual.iter().find(|p| p.period.year == y).unwrap().get(&StandardMetric::Revenue).unwrap()
    };
    assert_eq!(rev(2023), 211.9e9);
    assert_eq!(rev(2024), 245.1e9);
    assert_eq!(rev(2025), 281.7e9);
    assert_eq!(fin.annual.last().unwrap().end_date.as_deref(), Some("2025-06-30"));
}

#[test]
fn test_restated_value_from_latest_filing_wins() {
    let facts = facts_with_revenue(vec![
        fact(Some("2023-07-01"), "2024-06-30", 245.0e9, 2024, "FY", "10-K", "2024-07-30"),
        fact(Some("2023-07-01"), "2024-06-30", 245.1e9, 2025, "FY", "10-K", "2025-07-30"),
    ]);
    let fin = StandardizationEngine::new()
        .standardize(&facts, &company_with_fye("0630"))
        .unwrap();
    assert_eq!(fin.annual.len(), 1);
    assert_eq!(fin.annual[0].get(&StandardMetric::Revenue), Some(245.1e9));
}

#[test]
fn test_quarter_uses_three_month_value_not_year_to_date() {
    // A Q3 10-Q reports the 3-month and the 9-month figure with the same end date.
    let facts = facts_with_revenue(vec![
        fact(Some("2025-07-01"), "2025-09-30", 30.0e9, 2025, "Q3", "10-Q", "2025-10-30"),
        fact(Some("2025-01-01"), "2025-09-30", 85.0e9, 2025, "Q3", "10-Q", "2025-10-30"),
    ]);
    let fin = StandardizationEngine::new()
        .standardize(&facts, &company_with_fye("1231"))
        .unwrap();
    assert_eq!(fin.quarterly.len(), 1);
    assert_eq!(fin.quarterly[0].period.to_string(), FiscalPeriod::quarterly(2025, Quarter::Q3).to_string());
    assert_eq!(fin.quarterly[0].get(&StandardMetric::Revenue), Some(30.0e9));
    assert!(fin.annual.is_empty(), "a 9-month YTD figure is not an annual period");
}

#[test]
fn test_52_53_week_fiscal_year() {
    // NVDA-style: fiscal year ends the last Sunday of January; EDGAR FYE "0125".
    let facts = facts_with_revenue(vec![
        fact(Some("2024-01-29"), "2025-01-26", 130.5e9, 2025, "FY", "10-K", "2025-02-26"),
        fact(Some("2023-01-30"), "2024-01-28", 60.9e9, 2025, "FY", "10-K", "2025-02-26"),
        fact(Some("2024-10-28"), "2025-01-26", 39.3e9, 2025, "FY", "10-K", "2025-02-26"),
    ]);
    let fin = StandardizationEngine::new()
        .standardize(&facts, &company_with_fye("0125"))
        .unwrap();
    let years: Vec<(i32, Option<f64>)> = fin
        .annual
        .iter()
        .map(|p| (p.period.year, p.get(&StandardMetric::Revenue)))
        .collect();
    assert_eq!(years, vec![(2024, Some(60.9e9)), (2025, Some(130.5e9))]);
    // The discrete 13-week Q4 is a quarter, not a year.
    assert_eq!(fin.quarterly.len(), 1);
    assert_eq!(fin.quarterly[0].get(&StandardMetric::Revenue), Some(39.3e9));
}

// ── Revenue / FCF resolution ───────────────────────────────────────────

fn facts_with_tags(tags: Vec<(&str, f64)>) -> CompanyFactsResponse {
    let mut gaap = serde_json::Map::new();
    for (tag, val) in tags {
        gaap.insert(
            tag.to_string(),
            serde_json::json!({ "label": tag, "description": null, "units": { "USD": [
                fact(Some("2025-01-01"), "2025-12-31", val, 2025, "FY", "10-K", "2026-02-20")
            ]}}),
        );
    }
    serde_json::from_value(serde_json::json!({
        "cik": 1053507, "entityName": "Test Co", "facts": { "us-gaap": gaap }
    }))
    .unwrap()
}

fn fy2025(facts: CompanyFactsResponse) -> edgar_lib::standardizer::output::PeriodData {
    let fin = StandardizationEngine::new()
        .standardize(&facts, &company_with_fye("1231"))
        .unwrap();
    fin.annual.into_iter().find(|p| p.period.year == 2025).unwrap()
}

#[test]
fn test_lessor_revenue_uses_total_not_contract_revenue() {
    // American Tower style: ASC 606 contract revenue excludes lease revenue.
    let p = fy2025(facts_with_tags(vec![
        ("RevenueFromContractWithCustomerExcludingAssessedTax", 0.94e9),
        ("Revenues", 10.4e9),
    ]));
    assert_eq!(p.get(&StandardMetric::Revenue), Some(10.4e9));
    assert_eq!(
        p.metrics[&StandardMetric::Revenue].source_tag.as_deref(),
        Some("us-gaap:Revenues")
    );
}

#[test]
fn test_revenue_falls_back_when_no_total_is_tagged() {
    let p = fy2025(facts_with_tags(vec![("SalesRevenueGoodsNet", 5.0e9)]));
    assert_eq!(p.get(&StandardMetric::Revenue), Some(5.0e9));
}

#[test]
fn test_fcf_needs_capex() {
    let without = fy2025(facts_with_tags(vec![
        ("NetCashProvidedByUsedInOperatingActivities", 932e6),
        ("Revenues", 1.19e9),
    ]));
    assert_eq!(without.get(&StandardMetric::FreeCashFlow), None);
    assert_eq!(without.get(&StandardMetric::FreeCashFlowMargin), None);

    let with = fy2025(facts_with_tags(vec![
        ("NetCashProvidedByUsedInOperatingActivities", 932e6),
        ("PaymentsToAcquirePropertyPlantAndEquipment", 170e6),
        ("Revenues", 1.19e9),
    ]));
    assert_eq!(with.get(&StandardMetric::FreeCashFlow), Some(762e6));
}
