use edgar_lib::models::cik::Cik;
use edgar_lib::models::filing_type::FilingType;
use edgar_lib::models::period::{CalendarPeriod, Quarter};
use edgar_lib::models::ticker::{CompanyTicker, TickerMap};
use std::str::FromStr;

#[test]
fn test_cik_roundtrip() {
    let cik = Cik::new(320193).unwrap();
    assert_eq!(cik.as_u64(), 320193);
    assert_eq!(cik.zero_padded(), "0000320193");
    assert_eq!(cik.to_string(), "0000320193");

    // JSON roundtrip
    let json = serde_json::to_string(&cik).unwrap();
    let parsed: Cik = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, cik);
}

#[test]
fn test_cik_from_string_variants() {
    // Plain number
    assert_eq!(Cik::from_str("320193").unwrap().as_u64(), 320193);
    // Zero-padded
    assert_eq!(Cik::from_str("0000320193").unwrap().as_u64(), 320193);
    // With CIK prefix
    assert_eq!(Cik::from_str("CIK0000320193").unwrap().as_u64(), 320193);
}

#[test]
fn test_filing_type_parse() {
    assert_eq!(FilingType::from_str("10-K").unwrap(), FilingType::TenK);
    assert_eq!(FilingType::from_str("10-Q").unwrap(), FilingType::TenQ);
    assert_eq!(FilingType::from_str("8-K").unwrap(), FilingType::EightK);
    assert_eq!(FilingType::from_str("20-F").unwrap(), FilingType::TwentyF);

    // Lenient parsing for unknown types
    let unknown = FilingType::parse_lenient("NPORT-P");
    assert!(matches!(unknown, FilingType::Other(_)));
}

#[test]
fn test_filing_type_classification() {
    assert!(FilingType::TenK.is_annual());
    assert!(FilingType::TwentyF.is_annual());
    assert!(FilingType::TenQ.is_quarterly());
    assert!(FilingType::TenK.is_periodic());
    assert!(!FilingType::EightK.is_periodic());
    assert!(FilingType::TenKA.is_amendment());
}

#[test]
fn test_calendar_period_parse() {
    let annual = CalendarPeriod::from_str("CY2023").unwrap();
    assert_eq!(annual.year, 2023);
    assert!(annual.quarter.is_none());
    assert!(!annual.instantaneous);

    let quarterly = CalendarPeriod::from_str("CY2023Q1").unwrap();
    assert_eq!(quarterly.year, 2023);
    assert_eq!(quarterly.quarter, Some(Quarter::Q1));

    let instant = CalendarPeriod::from_str("CY2023Q4I").unwrap();
    assert!(instant.instantaneous);
    assert_eq!(instant.quarter, Some(Quarter::Q4));
}

#[test]
fn test_calendar_period_display() {
    assert_eq!(CalendarPeriod::annual(2023).to_string(), "CY2023");
    assert_eq!(
        CalendarPeriod::quarterly(2023, Quarter::Q2).to_string(),
        "CY2023Q2"
    );
    assert_eq!(
        CalendarPeriod::quarterly(2023, Quarter::Q4)
            .instantaneous()
            .to_string(),
        "CY2023Q4I"
    );
}

#[test]
fn test_ticker_map() {
    let entries = vec![
        CompanyTicker {
            cik_str: Cik::from(320193),
            ticker: "AAPL".to_string(),
            title: "Apple Inc.".to_string(),
        },
        CompanyTicker {
            cik_str: Cik::from(789019),
            ticker: "MSFT".to_string(),
            title: "MICROSOFT CORP".to_string(),
        },
    ];

    let map = TickerMap::from_entries(entries);
    assert_eq!(map.len(), 2);

    // Lookup by ticker (case-insensitive)
    let apple = map.lookup_ticker("aapl").unwrap();
    assert_eq!(apple.cik_str.as_u64(), 320193);
    assert_eq!(apple.title, "Apple Inc.");

    // Lookup by CIK
    let msft = map.lookup_cik(Cik::from(789019)).unwrap();
    assert_eq!(msft.ticker, "MSFT");

    // Not found
    assert!(map.lookup_ticker("GOOG").is_none());
}

#[test]
fn test_ticker_map_from_json() {
    let json = include_str!("fixtures/company_tickers_sample.json");
    let raw: std::collections::HashMap<String, CompanyTicker> =
        serde_json::from_str(json).unwrap();
    let entries: Vec<CompanyTicker> = raw.into_values().collect();
    let map = TickerMap::from_entries(entries);

    assert_eq!(map.len(), 3);
    assert!(map.lookup_ticker("AAPL").is_some());
    assert!(map.lookup_ticker("MSFT").is_some());
    assert!(map.lookup_ticker("AMZN").is_some());
}
