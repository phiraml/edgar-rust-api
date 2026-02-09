use edgar_lib::models::company_facts::FactValue;
use edgar_lib::standardizer::dedup::dedup_facts;

fn make_fact(end: &str, filed: &str, val: f64, form: &str, frame: Option<&str>) -> FactValue {
    FactValue {
        filed: filed.to_string(),
        start: None,
        end: end.to_string(),
        val: Some(val),
        accession: "test-accn".to_string(),
        form: Some(form.to_string()),
        fiscal_year: None,
        fiscal_period: None,
        frame: frame.map(|s| s.to_string()),
    }
}

#[test]
fn test_dedup_prefers_framed() {
    let facts = vec![
        make_fact("2023-09-30", "2023-11-01", 100.0, "10-K", None),
        make_fact("2023-09-30", "2023-11-03", 200.0, "10-K", Some("CY2023")),
    ];

    let deduped = dedup_facts(&facts);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].val, Some(200.0)); // Framed one wins
}

#[test]
fn test_dedup_prefers_latest_filed() {
    let facts = vec![
        make_fact("2023-09-30", "2023-11-01", 100.0, "10-K", None),
        make_fact("2023-09-30", "2023-12-15", 150.0, "10-K/A", None),
    ];

    let deduped = dedup_facts(&facts);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].val, Some(150.0)); // Later filing wins
}

#[test]
fn test_dedup_prefers_periodic_form() {
    let facts = vec![
        make_fact("2023-09-30", "2023-11-01", 100.0, "8-K", None),
        make_fact("2023-09-30", "2023-10-30", 200.0, "10-K", None),
    ];

    let deduped = dedup_facts(&facts);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].val, Some(200.0)); // 10-K wins over 8-K
}

#[test]
fn test_dedup_different_periods_kept() {
    let facts = vec![
        make_fact("2022-09-24", "2022-10-28", 100.0, "10-K", Some("CY2022")),
        make_fact("2023-09-30", "2023-11-03", 200.0, "10-K", Some("CY2023")),
    ];

    let deduped = dedup_facts(&facts);
    assert_eq!(deduped.len(), 2); // Different end dates => both kept
}

#[test]
fn test_dedup_sorted_by_end_date() {
    let facts = vec![
        make_fact("2023-09-30", "2023-11-03", 200.0, "10-K", Some("CY2023")),
        make_fact("2021-09-25", "2021-10-29", 50.0, "10-K", Some("CY2021")),
        make_fact("2022-09-24", "2022-10-28", 100.0, "10-K", Some("CY2022")),
    ];

    let deduped = dedup_facts(&facts);
    assert_eq!(deduped.len(), 3);
    assert!(deduped[0].end < deduped[1].end);
    assert!(deduped[1].end < deduped[2].end);
}
