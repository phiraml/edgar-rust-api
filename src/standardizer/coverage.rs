use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::models::company::Company;
use crate::models::company_facts::CompanyFactsResponse;
use crate::models::period::FiscalPeriod;
use crate::standardizer::catalog::{DefaultCatalog, MetricCatalog, MetricDefinition};
use crate::standardizer::catalog::sector::sector_definitions;
use crate::standardizer::output::{StandardMetric, StandardizedFinancials};
use crate::standardizer::resolution::MetricResolution;

/// A candidate XBRL tag that might map to a missing standard metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateTag {
    pub taxonomy: String,
    pub tag: String,
    pub label: Option<String>,
    pub latest_value: Option<f64>,
    pub unit: String,
    pub match_reason: String,
}

/// A standard metric that could not be resolved from the company's XBRL data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingMetric {
    pub metric: StandardMetric,
    pub display_name: String,
    pub tags_tried: Vec<String>,
    pub candidates: Vec<CandidateTag>,
}

/// A gap between a total metric and the sum of its known components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementGap {
    pub total_metric: StandardMetric,
    pub total_value: f64,
    pub known_components: Vec<(StandardMetric, f64)>,
    pub known_sum: f64,
    pub unexplained_amount: f64,
    pub unexplained_pct: f64,
}

/// The complete coverage analysis report for a company.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub entity_name: String,
    pub period: FiscalPeriod,
    pub expected_count: usize,
    pub resolved_count: usize,
    pub coverage_pct: f64,
    pub missing_metrics: Vec<MissingMetric>,
    pub statement_gaps: Vec<StatementGap>,
}

pub struct CoverageAnalyzer;

impl CoverageAnalyzer {
    /// Analyze coverage gaps for a company's latest annual period.
    ///
    /// If `override_definitions` is provided, uses those definitions instead of
    /// the default catalog. This allows callers to pass augmented definitions
    /// (e.g., with learned tags prepended).
    pub fn analyze(
        facts: &CompanyFactsResponse,
        company: &Company,
        financials: &StandardizedFinancials,
        override_definitions: Option<&[MetricDefinition]>,
    ) -> CoverageReport {
        let latest = financials.latest_annual();

        let definitions: Vec<MetricDefinition> = match override_definitions {
            Some(defs) => defs.to_vec(),
            None => {
                let catalog = DefaultCatalog;
                let mut defs = catalog.definitions();
                defs.extend(sector_definitions(company.sic.as_deref()));
                defs
            }
        };

        // Filter to primary metrics (not Custom/ratio)
        let primary_defs: Vec<_> = definitions
            .iter()
            .filter(|d| !matches!(d.resolution, MetricResolution::Custom(_)))
            .collect();

        let expected_count = primary_defs.len();

        // Determine which metrics resolved and which are missing
        let mut resolved_count = 0;
        let mut missing_metrics = Vec::new();

        for def in &primary_defs {
            let resolved = latest
                .map(|p| p.metrics.contains_key(&def.metric))
                .unwrap_or(false);

            if resolved {
                resolved_count += 1;
            } else {
                // Collect the tags that were tried
                let tags_tried: Vec<String> = def
                    .resolution
                    .tag_specs()
                    .iter()
                    .map(|s| format!("{}:{}", s.taxonomy, s.tag))
                    .collect();

                // Search for keyword candidates
                let candidates = search_keyword_candidates(&def.metric, facts, &definitions);

                missing_metrics.push(MissingMetric {
                    metric: def.metric.clone(),
                    display_name: def.metric.display_name().to_string(),
                    tags_tried,
                    candidates,
                });
            }
        }

        let coverage_pct = if expected_count > 0 {
            (resolved_count as f64 / expected_count as f64) * 100.0
        } else {
            100.0
        };

        // Detect statement gaps
        let statement_gaps = if let Some(period_data) = latest {
            detect_statement_gaps(period_data)
        } else {
            vec![]
        };

        let period = latest
            .map(|p| p.period.clone())
            .unwrap_or_else(|| FiscalPeriod::annual(0));

        CoverageReport {
            entity_name: facts.entity_name.clone(),
            period,
            expected_count,
            resolved_count,
            coverage_pct,
            missing_metrics,
            statement_gaps,
        }
    }
}

/// Keyword fragments for each primary metric, used to find candidate tags.
fn search_keywords(metric: &StandardMetric) -> Vec<&'static str> {
    match metric {
        StandardMetric::Revenue => vec!["revenue", "sales", "net sales"],
        StandardMetric::CostOfRevenue => vec!["cost of revenue", "cost of goods", "cost of sales", "cogs"],
        StandardMetric::GrossProfit => vec!["gross profit"],
        StandardMetric::ResearchAndDevelopment => vec![
            "research", "development", "r&d", "technology", "product development",
        ],
        StandardMetric::SellingGeneralAdmin => vec![
            "selling", "general and administrative", "sg&a", "marketing",
            "administrative",
        ],
        StandardMetric::OperatingExpenses => vec!["operating expense", "total operating"],
        StandardMetric::OperatingIncome => vec!["operating income", "operating loss", "income from operations"],
        StandardMetric::InterestExpense => vec!["interest expense", "interest cost"],
        StandardMetric::InterestIncome => vec!["interest income", "investment income"],
        StandardMetric::OtherNonOperatingIncome => vec!["non-operating", "nonoperating", "other income"],
        StandardMetric::PretaxIncome => vec!["pretax", "pre-tax", "before income tax", "before tax"],
        StandardMetric::IncomeTaxExpense => vec!["income tax", "tax expense", "tax benefit"],
        StandardMetric::NetIncome => vec!["net income", "net loss", "net earnings"],
        StandardMetric::NetIncomeToCommon => vec!["net income", "common stockholder", "common shareholder"],
        StandardMetric::DepreciationAmortization => vec!["depreciation", "amortization", "d&a"],
        StandardMetric::CashAndEquivalents => vec!["cash", "cash equivalent"],
        StandardMetric::ShortTermInvestments => vec!["short-term investment", "marketable securities"],
        StandardMetric::CashAndShortTermInvestments => vec!["cash and short-term", "cash and investment"],
        StandardMetric::AccountsReceivable => vec!["accounts receivable", "receivable", "trade receivable"],
        StandardMetric::Inventory => vec!["inventory", "inventories"],
        StandardMetric::OtherCurrentAssets => vec!["other current asset", "prepaid"],
        StandardMetric::TotalCurrentAssets => vec!["current assets", "total current asset"],
        StandardMetric::PropertyPlantEquipment => vec!["property", "plant", "equipment", "pp&e", "ppe"],
        StandardMetric::Goodwill => vec!["goodwill"],
        StandardMetric::IntangibleAssets => vec!["intangible"],
        StandardMetric::OtherNonCurrentAssets => vec!["other non-current", "other noncurrent", "other asset"],
        StandardMetric::TotalAssets => vec!["total assets", "assets"],
        StandardMetric::AccountsPayable => vec!["accounts payable", "trade payable"],
        StandardMetric::ShortTermDebt => vec!["short-term debt", "short-term borrowing", "commercial paper"],
        StandardMetric::CurrentPortionLongTermDebt => vec!["current portion", "long-term debt current"],
        StandardMetric::OtherCurrentLiabilities => vec!["other current liabilit", "accrued liabilit"],
        StandardMetric::TotalCurrentLiabilities => vec!["current liabilities", "total current liabilit"],
        StandardMetric::LongTermDebt => vec!["long-term debt", "long term debt"],
        StandardMetric::OtherNonCurrentLiabilities => vec!["other non-current liabilit", "other noncurrent liabilit"],
        StandardMetric::TotalLiabilities => vec!["total liabilities"],
        StandardMetric::CommonStock => vec!["common stock"],
        StandardMetric::RetainedEarnings => vec!["retained earnings", "accumulated deficit"],
        StandardMetric::AccumulatedOtherComprehensiveIncome => vec!["other comprehensive income", "oci"],
        StandardMetric::TotalStockholdersEquity => vec!["stockholders equity", "shareholders equity", "total equity"],
        StandardMetric::TotalLiabilitiesAndEquity => vec!["liabilities and equity", "liabilities and stockholders"],
        StandardMetric::OperatingCashFlow => vec!["operating activities", "cash from operations"],
        StandardMetric::CapitalExpenditures => vec!["capital expenditure", "purchase of property", "capex"],
        StandardMetric::InvestingCashFlow => vec!["investing activities"],
        StandardMetric::FinancingCashFlow => vec!["financing activities"],
        StandardMetric::DividendsPaid => vec!["dividend", "distribution"],
        StandardMetric::ShareRepurchases => vec!["repurchase", "buyback", "treasury stock"],
        StandardMetric::NetChangeInCash => vec!["change in cash", "increase decrease"],
        StandardMetric::EarningsPerShareBasic => vec!["earnings per share", "eps", "basic"],
        StandardMetric::EarningsPerShareDiluted => vec!["earnings per share", "eps", "diluted"],
        StandardMetric::DividendsPerShare => vec!["dividend per share"],
        StandardMetric::SharesOutstandingBasic => vec!["shares outstanding", "weighted average"],
        StandardMetric::SharesOutstandingDiluted => vec!["diluted shares", "weighted average diluted"],
        _ => vec![],
    }
}

/// Search company facts for tags whose labels match keyword fragments for a missing metric.
fn search_keyword_candidates(
    metric: &StandardMetric,
    facts: &CompanyFactsResponse,
    definitions: &[MetricDefinition],
) -> Vec<CandidateTag> {
    let keywords = search_keywords(metric);
    if keywords.is_empty() {
        return vec![];
    }

    // Collect all tags already known to the catalog so we can skip them
    let known_tags: HashSet<String> = definitions
        .iter()
        .flat_map(|d| {
            d.resolution
                .tag_specs()
                .into_iter()
                .map(|s| format!("{}:{}", s.taxonomy, s.tag))
        })
        .collect();

    let mut candidates = Vec::new();

    for (taxonomy, tags) in &facts.facts {
        for (tag, tag_data) in tags {
            let key = format!("{taxonomy}:{tag}");
            if known_tags.contains(&key) {
                continue;
            }

            // Check label and tag name against keywords
            let label_lower = tag_data
                .label
                .as_deref()
                .unwrap_or("")
                .to_lowercase();
            let tag_lower = tag.to_lowercase();

            for keyword in &keywords {
                let kw = keyword.to_lowercase();
                if label_lower.contains(&kw) || tag_lower.contains(&kw) {
                    // Find the latest value and its unit
                    let (latest_value, unit) = find_latest_value(tag_data);

                    candidates.push(CandidateTag {
                        taxonomy: taxonomy.clone(),
                        tag: tag.clone(),
                        label: tag_data.label.clone(),
                        latest_value,
                        unit,
                        match_reason: format!("keyword: {keyword}"),
                    });
                    break; // Don't add the same tag multiple times
                }
            }
        }
    }

    // Sort by value descending (largest values first — more likely to be the right match)
    candidates.sort_by(|a, b| {
        b.latest_value
            .unwrap_or(0.0)
            .partial_cmp(&a.latest_value.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    candidates
}

/// Find the latest annual value and its unit from tag data.
fn find_latest_value(tag_data: &crate::models::company_facts::TagData) -> (Option<f64>, String) {
    let mut best: Option<(f64, String)> = None;
    let mut best_end = String::new();

    for (unit, values) in &tag_data.units {
        for fact in values {
            if fact.is_annual() {
                if let Some(val) = fact.val {
                    if fact.end > best_end {
                        best_end = fact.end.clone();
                        best = Some((val, unit.clone()));
                    }
                }
            }
        }
    }

    match best {
        Some((val, unit)) => (Some(val), unit),
        None => (None, "unknown".to_string()),
    }
}

/// Detect statement-level gaps (e.g., OpEx vs sum of components).
fn detect_statement_gaps(
    period_data: &crate::standardizer::output::PeriodData,
) -> Vec<StatementGap> {
    let mut gaps = Vec::new();

    // OpEx gap: OperatingExpenses vs (CostOfRevenue + R&D + SG&A)
    if let Some(opex) = period_data.get(&StandardMetric::OperatingExpenses) {
        let components = vec![
            StandardMetric::CostOfRevenue,
            StandardMetric::ResearchAndDevelopment,
            StandardMetric::SellingGeneralAdmin,
            StandardMetric::DepreciationAmortization,
        ];
        check_gap(&mut gaps, period_data, StandardMetric::OperatingExpenses, opex, &components);
    }

    // Revenue - OpEx = Operating Income check
    // Total Assets vs components
    if let Some(total_assets) = period_data.get(&StandardMetric::TotalAssets) {
        let components = vec![
            StandardMetric::TotalCurrentAssets,
            StandardMetric::PropertyPlantEquipment,
            StandardMetric::Goodwill,
            StandardMetric::IntangibleAssets,
            StandardMetric::OtherNonCurrentAssets,
        ];
        check_gap(&mut gaps, period_data, StandardMetric::TotalAssets, total_assets, &components);
    }

    // Total Current Assets vs components
    if let Some(total_ca) = period_data.get(&StandardMetric::TotalCurrentAssets) {
        let components = vec![
            StandardMetric::CashAndEquivalents,
            StandardMetric::ShortTermInvestments,
            StandardMetric::AccountsReceivable,
            StandardMetric::Inventory,
            StandardMetric::OtherCurrentAssets,
        ];
        check_gap(&mut gaps, period_data, StandardMetric::TotalCurrentAssets, total_ca, &components);
    }

    // Total Liabilities vs components
    if let Some(total_liab) = period_data.get(&StandardMetric::TotalLiabilities) {
        let components = vec![
            StandardMetric::TotalCurrentLiabilities,
            StandardMetric::LongTermDebt,
            StandardMetric::OtherNonCurrentLiabilities,
        ];
        check_gap(&mut gaps, period_data, StandardMetric::TotalLiabilities, total_liab, &components);
    }

    gaps
}

fn check_gap(
    gaps: &mut Vec<StatementGap>,
    period_data: &crate::standardizer::output::PeriodData,
    total_metric: StandardMetric,
    total_value: f64,
    component_metrics: &[StandardMetric],
) {
    let mut known_components = Vec::new();
    let mut known_sum = 0.0;

    for m in component_metrics {
        if let Some(val) = period_data.get(m) {
            known_components.push((m.clone(), val));
            known_sum += val;
        }
    }

    if known_components.is_empty() {
        return;
    }

    let unexplained = total_value - known_sum;
    let unexplained_pct = if total_value.abs() > 1e-10 {
        (unexplained / total_value).abs() * 100.0
    } else {
        0.0
    };

    // Only report gaps above 5% of the total
    if unexplained_pct > 5.0 {
        gaps.push(StatementGap {
            total_metric,
            total_value,
            known_components,
            known_sum,
            unexplained_amount: unexplained,
            unexplained_pct,
        });
    }
}
