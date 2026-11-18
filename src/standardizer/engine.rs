use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDate;

use crate::error::Result;
use crate::models::company::Company;
use crate::models::company_facts::{CompanyFactsResponse, FactValue};
use crate::models::period::{FiscalPeriod, Quarter};

use super::catalog::sector::sector_definitions;
use super::catalog::{DefaultCatalog, MetricCatalog};
use super::dedup::is_periodic_form;
use super::output::{MetricValue, PeriodData, StandardMetric, StandardizedFinancials};
use super::period_align::{fact_periods, FiscalYearEnd};
use super::resolution::{MetricResolution, TagSpec};

type FactKey = (String, String, String);

/// Index over a company's XBRL facts for fast lookup by taxonomy+tag+unit.
#[derive(Debug)]
pub struct FactIndex {
    /// Keyed by (taxonomy, tag, unit) → raw facts as returned by EDGAR.
    facts: HashMap<FactKey, Vec<FactValue>>,
    /// Per tag: index into `facts[key]` of the best fact for each fiscal period,
    /// chosen once here so per-metric resolution is a map lookup.
    best_by_period: HashMap<FactKey, HashMap<FiscalPeriod, usize>>,
    /// Periods that have real data, with the reported end date closest to the
    /// nominal period end (so a cover-page date weeks later doesn't win).
    annual_ends: BTreeMap<i32, (i64, NaiveDate)>,
    quarterly_ends: BTreeMap<(i32, Quarter), (i64, NaiveDate)>,
    /// Already-resolved metric values for the current period (used by ratio computations).
    resolved_metrics: HashMap<StandardMetric, f64>,
}

impl FactIndex {
    pub fn build(facts_response: &CompanyFactsResponse, company: &Company) -> Self {
        let fye = FiscalYearEnd::from_company(company);
        let mut facts: HashMap<FactKey, Vec<FactValue>> = HashMap::new();
        let mut best_by_period = HashMap::new();
        let mut annual_ends: BTreeMap<i32, (i64, NaiveDate)> = BTreeMap::new();
        let mut quarterly_ends: BTreeMap<(i32, Quarter), (i64, NaiveDate)> = BTreeMap::new();
        // Closest to the nominal end wins; ties go to the later date.
        let closer = |slot: &mut (i64, NaiveDate), cand: (i64, NaiveDate)| {
            if (Reverse(cand.0), cand.1) > (Reverse(slot.0), slot.1) {
                *slot = cand;
            }
        };

        for (taxonomy, tags) in &facts_response.facts {
            for (tag, tag_data) in tags {
                for (unit, values) in &tag_data.units {
                    let key = (taxonomy.clone(), tag.clone(), unit.clone());

                    // Best fact per period: closest to the period end, then from a
                    // periodic report, then most recently filed (restated values).
                    let mut best: HashMap<FiscalPeriod, (usize, (Reverse<i64>, bool, &str))> =
                        HashMap::new();
                    for (i, fact) in values.iter().enumerate() {
                        if fact.val.is_none() {
                            continue;
                        }
                        let (Some(fp), Some(end)) = (fact_periods(fact, &fye), fact.end_date())
                        else {
                            continue;
                        };
                        let cand = (fp.distance_days, end);
                        if let (true, Some(y)) = (fp.creates_annual, fp.annual) {
                            closer(annual_ends.entry(y).or_insert(cand), cand);
                        }
                        if let (true, Some(yq)) = (fp.creates_quarter, fp.quarter) {
                            closer(quarterly_ends.entry(yq).or_insert(cand), cand);
                        }

                        let rank = (
                            Reverse(fp.distance_days),
                            is_periodic_form(fact),
                            fact.filed.as_str(),
                        );
                        let periods = fp
                            .annual
                            .map(FiscalPeriod::annual)
                            .into_iter()
                            .chain(fp.quarter.map(|(y, q)| FiscalPeriod::quarterly(y, q)));
                        for period in periods {
                            match best.get(&period) {
                                Some((_, current)) if *current >= rank => {}
                                _ => {
                                    best.insert(period, (i, rank));
                                }
                            }
                        }
                    }
                    let best: HashMap<FiscalPeriod, usize> =
                        best.into_iter().map(|(p, (i, _))| (p, i)).collect();
                    best_by_period.insert(key.clone(), best);
                    facts.insert(key, values.clone());
                }
            }
        }

        Self {
            facts,
            best_by_period,
            annual_ends,
            quarterly_ends,
            resolved_metrics: HashMap::new(),
        }
    }

    /// The best fact for `period` under `spec`. With no unit on the spec, the
    /// first common unit the tag has is used (same rule as `lookup_tag`).
    fn fact_for_period(&self, spec: &TagSpec, period: &FiscalPeriod) -> Option<&FactValue> {
        const COMMON_UNITS: [&str; 4] = ["USD", "shares", "USD/shares", "pure"];
        let explicit;
        let units: &[&str] = match &spec.unit {
            Some(u) => {
                explicit = [u.as_str()];
                &explicit
            }
            None => &COMMON_UNITS,
        };
        for unit in units {
            let key = (spec.taxonomy.clone(), spec.tag.clone(), unit.to_string());
            if let Some(facts) = self.facts.get(&key) {
                let i = *self.best_by_period.get(&key)?.get(period)?;
                return facts.get(i);
            }
        }
        None
    }

    /// Latest reported end date for a period that has data.
    pub fn period_end(&self, period: &FiscalPeriod) -> Option<NaiveDate> {
        match period.quarter {
            None => self.annual_ends.get(&period.year).map(|&(_, d)| d),
            Some(q) => self.quarterly_ends.get(&(period.year, q)).map(|&(_, d)| d),
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

    /// Fiscal years with annual data, oldest first.
    pub fn annual_periods(&self) -> Vec<FiscalPeriod> {
        self.annual_ends.keys().map(|&y| FiscalPeriod::annual(y)).collect()
    }

    /// Fiscal quarters with quarterly (3-month) data, oldest first.
    pub fn quarterly_periods(&self) -> Vec<FiscalPeriod> {
        self.quarterly_ends
            .keys()
            .map(|&(y, q)| FiscalPeriod::quarterly(y, q))
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
                    if let Some(val) = self.fact_for_period(spec, period).and_then(|f| f.val) {
                        return Some((val, Some(format!("{}:{}", spec.taxonomy, spec.tag))));
                    }
                }
                None
            }

            MetricResolution::Largest(specs) => specs
                .iter()
                .filter_map(|spec| {
                    let val = self.fact_for_period(spec, period)?.val?;
                    Some((val, spec))
                })
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(val, spec)| (val, Some(format!("{}:{}", spec.taxonomy, spec.tag)))),

            MetricResolution::FirstOf(options) => options
                .iter()
                .find_map(|resolution| self.resolve_for_period(resolution, period)),

            MetricResolution::Sum(specs) => {
                let mut total = 0.0;
                let mut found_any = false;
                let mut source = String::new();
                for spec in specs {
                    if let Some(val) = self.fact_for_period(spec, period).and_then(|f| f.val) {
                        total += val;
                        found_any = true;
                        if !source.is_empty() {
                            source.push_str(" + ");
                        }
                        source.push_str(&format!("{}:{}", spec.taxonomy, spec.tag));
                    }
                }
                if found_any {
                    Some((total, Some(source)))
                } else {
                    None
                }
            }

            MetricResolution::Difference(a_spec, b_spec) => {
                let a_val = self.fact_for_period(a_spec, period).and_then(|f| f.val)?;
                let b_val = self.fact_for_period(b_spec, period).and_then(|f| f.val)?;
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
                    end_date: index.period_end(period).map(|d| d.to_string()),
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
                    end_date: index.period_end(period).map(|d| d.to_string()),
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
