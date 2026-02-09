use std::collections::{BTreeMap, HashMap};

use crate::error::Result;
use crate::models::company::Company;
use crate::models::company_facts::{CompanyFactsResponse, FactValue};
use crate::models::period::FiscalPeriod;

use super::catalog::sector::sector_definitions;
use super::catalog::{DefaultCatalog, MetricCatalog};
use super::dedup::dedup_facts;
use super::output::{MetricValue, PeriodData, StandardMetric, StandardizedFinancials};
use super::period_align::detect_fiscal_year_end;
use super::resolution::{MetricResolution, TagSpec};

/// Index over a company's XBRL facts for fast lookup by taxonomy+tag+unit.
#[derive(Debug)]
pub struct FactIndex {
    /// Keyed by (taxonomy, tag, unit) → deduplicated, sorted facts.
    facts: HashMap<(String, String, String), Vec<FactValue>>,
    /// Already-resolved metric values for the current period (used by ratio computations).
    resolved_metrics: HashMap<StandardMetric, f64>,
    /// Fiscal year end month.
    #[allow(dead_code)]
    fy_end_month: u32,
}

impl FactIndex {
    pub fn build(facts_response: &CompanyFactsResponse, company: &Company) -> Self {
        let fy_end_month = detect_fiscal_year_end(company);
        let mut facts: HashMap<(String, String, String), Vec<FactValue>> = HashMap::new();

        for (taxonomy, tags) in &facts_response.facts {
            for (tag, tag_data) in tags {
                for (unit, values) in &tag_data.units {
                    let key = (taxonomy.clone(), tag.clone(), unit.clone());
                    facts.insert(key, values.clone());
                }
            }
        }

        Self {
            facts,
            resolved_metrics: HashMap::new(),
            fy_end_month,
        }
    }

    /// Look up facts for a specific taxonomy/tag/unit combination.
    pub fn lookup(&self, taxonomy: &str, tag: &str, unit: &str) -> Option<&[FactValue]> {
        self.facts
            .get(&(taxonomy.to_string(), tag.to_string(), unit.to_string()))
            .map(|v| v.as_slice())
    }

    /// Look up facts matching a TagSpec. If unit is None, try common units.
    pub fn lookup_tag(&self, spec: &TagSpec) -> Option<&[FactValue]> {
        if let Some(ref unit) = spec.unit {
            return self.lookup(&spec.taxonomy, &spec.tag, unit);
        }
        // Try common units
        for unit in &["USD", "shares", "USD/shares", "pure"] {
            if let Some(facts) = self.lookup(&spec.taxonomy, &spec.tag, unit) {
                return Some(facts);
            }
        }
        None
    }

    /// Get a resolved metric value (used by ratio computations).
    pub fn resolved(&self, metric: StandardMetric) -> Option<f64> {
        self.resolved_metrics.get(&metric).copied()
    }

    /// Set a resolved metric value.
    pub fn set_resolved(&mut self, metric: StandardMetric, value: f64) {
        self.resolved_metrics.insert(metric, value);
    }

    /// Clear resolved metrics for a new period.
    pub fn clear_resolved(&mut self) {
        self.resolved_metrics.clear();
    }

    /// Get all unique fiscal periods present in the data for annual reporting.
    pub fn annual_periods(&self) -> Vec<FiscalPeriod> {
        let mut periods = std::collections::BTreeSet::new();

        for facts in self.facts.values() {
            for fact in facts {
                if fact.is_annual() {
                    if let Some(fy) = fact.fiscal_year {
                        periods.insert(fy);
                    }
                }
            }
        }

        periods
            .into_iter()
            .map(FiscalPeriod::annual)
            .collect()
    }

    /// Get all unique fiscal periods for quarterly reporting.
    pub fn quarterly_periods(&self) -> Vec<FiscalPeriod> {
        use crate::models::period::Quarter;
        let mut periods = std::collections::BTreeSet::new();

        for facts in self.facts.values() {
            for fact in facts {
                if fact.is_quarterly() {
                    if let (Some(fy), Some(fp)) = (fact.fiscal_year, fact.fiscal_period.as_deref()) {
                        let q = match fp {
                            "Q1" => Some(Quarter::Q1),
                            "Q2" => Some(Quarter::Q2),
                            "Q3" => Some(Quarter::Q3),
                            "Q4" => Some(Quarter::Q4),
                            _ => None,
                        };
                        if let Some(q) = q {
                            periods.insert((fy, q));
                        }
                    }
                }
            }
        }

        periods
            .into_iter()
            .map(|(y, q)| FiscalPeriod::quarterly(y, q))
            .collect()
    }

    /// Resolve a metric for a specific period using the given resolution strategy.
    pub fn resolve_for_period(
        &self,
        resolution: &MetricResolution,
        period: &FiscalPeriod,
    ) -> Option<(f64, Option<String>)> {
        match resolution {
            MetricResolution::FirstMatch(specs) => {
                for spec in specs {
                    if let Some(facts) = self.lookup_tag(spec) {
                        let deduped = dedup_facts(facts);
                        let matching = find_period_match(&deduped, period);
                        if let Some(fact) = matching {
                            if let Some(val) = fact.val {
                                return Some((val, Some(format!("{}:{}", spec.taxonomy, spec.tag))));
                            }
                        }
                    }
                }
                None
            }

            MetricResolution::Sum(specs) => {
                let mut total = 0.0;
                let mut found_any = false;
                let mut source = String::new();
                for spec in specs {
                    if let Some(facts) = self.lookup_tag(spec) {
                        let deduped = dedup_facts(facts);
                        if let Some(fact) = find_period_match(&deduped, period) {
                            if let Some(val) = fact.val {
                                total += val;
                                found_any = true;
                                if !source.is_empty() {
                                    source.push_str(" + ");
                                }
                                source.push_str(&format!("{}:{}", spec.taxonomy, spec.tag));
                            }
                        }
                    }
                }
                if found_any {
                    Some((total, Some(source)))
                } else {
                    None
                }
            }

            MetricResolution::Difference(a_spec, b_spec) => {
                let a_val = self.lookup_tag(a_spec).and_then(|facts| {
                    let deduped = dedup_facts(facts);
                    find_period_match(&deduped, period).and_then(|f| f.val)
                })?;
                let b_val = self.lookup_tag(b_spec).and_then(|facts| {
                    let deduped = dedup_facts(facts);
                    find_period_match(&deduped, period).and_then(|f| f.val)
                })?;
                Some((
                    a_val - b_val,
                    Some(format!(
                        "{}:{} - {}:{}",
                        a_spec.taxonomy, a_spec.tag, b_spec.taxonomy, b_spec.tag
                    )),
                ))
            }

            MetricResolution::Ratio(a_res, b_res) => {
                let (a_val, _) = self.resolve_for_period(a_res, period)?;
                let (b_val, _) = self.resolve_for_period(b_res, period)?;
                if b_val.abs() < 1e-10 {
                    return None;
                }
                Some((a_val / b_val, Some("ratio".to_string())))
            }

            MetricResolution::Custom(f) => {
                f(self).map(|v| (v, Some("computed".to_string())))
            }
        }
    }
}

/// Find the fact that best matches a fiscal period.
fn find_period_match<'a>(facts: &[&'a FactValue], period: &FiscalPeriod) -> Option<&'a FactValue> {
    let target_fy = period.year;
    let is_annual = period.quarter.is_none();

    let matching: Vec<&&FactValue> = facts
        .iter()
        .filter(|f| {
            let fy_match = f.fiscal_year == Some(target_fy);
            let period_match = if is_annual {
                f.fiscal_period.as_deref() == Some("FY")
                    || (f.form.as_deref() == Some("10-K") && f.fiscal_period.is_none())
            } else {
                let target_q = match period.quarter {
                    Some(crate::models::period::Quarter::Q1) => "Q1",
                    Some(crate::models::period::Quarter::Q2) => "Q2",
                    Some(crate::models::period::Quarter::Q3) => "Q3",
                    Some(crate::models::period::Quarter::Q4) => "Q4",
                    None => return false,
                };
                f.fiscal_period.as_deref() == Some(target_q)
            };
            fy_match && period_match
        })
        .collect();

    // Prefer framed facts
    let framed: Vec<&&FactValue> = matching.iter().filter(|f| f.frame.is_some()).copied().collect();
    if let Some(best) = framed.last() {
        return Some(best);
    }

    matching.last().copied().copied()
}

/// The standardization engine that converts raw XBRL facts into standardized financials.
pub struct StandardizationEngine {
    catalog: Box<dyn MetricCatalog>,
}

impl StandardizationEngine {
    pub fn new() -> Self {
        Self {
            catalog: Box::new(DefaultCatalog),
        }
    }

    pub fn with_catalog(catalog: Box<dyn MetricCatalog>) -> Self {
        Self { catalog }
    }

    pub fn standardize(
        &self,
        facts: &CompanyFactsResponse,
        company: &Company,
    ) -> Result<StandardizedFinancials> {
        let mut index = FactIndex::build(facts, company);

        let mut definitions = self.catalog.definitions();
        // Add sector-specific definitions
        definitions.extend(sector_definitions(company.sic.as_deref()));

        // Separate primary metrics from computed ones (ratios)
        let (primary_defs, computed_defs): (Vec<_>, Vec<_>) = definitions
            .into_iter()
            .partition(|d| !matches!(d.resolution, MetricResolution::Custom(_)));

        // Process annual periods
        let annual_periods = index.annual_periods();
        let mut annual = Vec::new();

        for period in &annual_periods {
            index.clear_resolved();
            let mut metrics = BTreeMap::new();

            // First pass: resolve primary metrics
            for def in &primary_defs {
                if let Some((value, source_tag)) = index.resolve_for_period(&def.resolution, period) {
                    let unit = infer_unit(&def.metric);
                    metrics.insert(
                        def.metric.clone(),
                        MetricValue {
                            metric: def.metric.clone(),
                            value,
                            unit,
                            source_tag,
                        },
                    );
                    index.set_resolved(def.metric.clone(), value);
                }
            }

            // Second pass: compute ratios from resolved metrics
            for def in &computed_defs {
                if let Some((value, source_tag)) = index.resolve_for_period(&def.resolution, period) {
                    let unit = if def.metric.is_ratio() {
                        "ratio".to_string()
                    } else {
                        infer_unit(&def.metric)
                    };
                    metrics.insert(
                        def.metric.clone(),
                        MetricValue {
                            metric: def.metric.clone(),
                            value,
                            unit,
                            source_tag,
                        },
                    );
                }
            }

            if !metrics.is_empty() {
                annual.push(PeriodData {
                    period: period.clone(),
                    end_date: None,
                    metrics,
                });
            }
        }

        // Process quarterly periods
        let quarterly_periods = index.quarterly_periods();
        let mut quarterly = Vec::new();

        for period in &quarterly_periods {
            index.clear_resolved();
            let mut metrics = BTreeMap::new();

            for def in &primary_defs {
                if let Some((value, source_tag)) = index.resolve_for_period(&def.resolution, period) {
                    let unit = infer_unit(&def.metric);
                    metrics.insert(
                        def.metric.clone(),
                        MetricValue {
                            metric: def.metric.clone(),
                            value,
                            unit,
                            source_tag,
                        },
                    );
                    index.set_resolved(def.metric.clone(), value);
                }
            }

            for def in &computed_defs {
                if let Some((value, source_tag)) = index.resolve_for_period(&def.resolution, period) {
                    let unit = if def.metric.is_ratio() {
                        "ratio".to_string()
                    } else {
                        infer_unit(&def.metric)
                    };
                    metrics.insert(
                        def.metric.clone(),
                        MetricValue {
                            metric: def.metric.clone(),
                            value,
                            unit,
                            source_tag,
                        },
                    );
                }
            }

            if !metrics.is_empty() {
                quarterly.push(PeriodData {
                    period: period.clone(),
                    end_date: None,
                    metrics,
                });
            }
        }

        Ok(StandardizedFinancials {
            entity_name: facts.entity_name.clone(),
            cik: facts.cik.as_u64(),
            annual,
            quarterly,
        })
    }
}

fn infer_unit(metric: &StandardMetric) -> String {
    match metric {
        StandardMetric::SharesOutstandingBasic
        | StandardMetric::SharesOutstandingDiluted => "shares".to_string(),
        StandardMetric::EarningsPerShareBasic
        | StandardMetric::EarningsPerShareDiluted
        | StandardMetric::DividendsPerShare
        | StandardMetric::BookValuePerShare
        | StandardMetric::RevenuePerShare
        | StandardMetric::FreeCashFlowPerShare => "USD/shares".to_string(),
        m if m.is_ratio() => "ratio".to_string(),
        _ => "USD".to_string(),
    }
}
