use chrono::{Datelike, NaiveDate};

use crate::models::company::Company;
use crate::models::company_facts::FactValue;
use crate::models::period::{FiscalPeriod, Quarter};

/// A company's fiscal year end as month/day (EDGAR's `fiscalYearEnd`, "MMDD").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FiscalYearEnd {
    pub month: u32,
    pub day: u32,
}

impl Default for FiscalYearEnd {
    fn default() -> Self {
        Self { month: 12, day: 31 }
    }
}

impl FiscalYearEnd {
    pub fn from_company(company: &Company) -> Self {
        company
            .fiscal_year_end
            .as_deref()
            .and_then(Self::parse)
            .unwrap_or_default()
    }

    pub fn parse(mmdd: &str) -> Option<Self> {
        if mmdd.len() != 4 {
            return None;
        }
        let month = mmdd[..2].parse::<u32>().ok()?;
        let day = mmdd[2..].parse::<u32>().ok()?;
        ((1..=12).contains(&month) && (1..=31).contains(&day)).then_some(Self { month, day })
    }

    /// Nominal end date of quarter `q` of fiscal year `year` (Q4 ends the fiscal year).
    /// 52/53-week filers land within a few days of this.
    pub fn quarter_end(&self, year: i32, q: Quarter) -> NaiveDate {
        let months_before_fye = match q {
            Quarter::Q1 => 9,
            Quarter::Q2 => 6,
            Quarter::Q3 => 3,
            Quarter::Q4 => 0,
        };
        let total = year * 12 + (self.month as i32 - 1) - months_before_fye;
        let (y, m) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
        let last_day = last_day_of_month(y, m);
        NaiveDate::from_ymd_opt(y, m, self.day.min(last_day)).expect("valid clamped date")
    }

    /// The fiscal quarter end nearest to `date`, with the distance in days.
    pub fn nearest_quarter_end(&self, date: NaiveDate) -> (i32, Quarter, i64) {
        let mut best = (date.year(), Quarter::Q4, i64::MAX);
        for year in date.year() - 1..=date.year() + 1 {
            for q in [Quarter::Q1, Quarter::Q2, Quarter::Q3, Quarter::Q4] {
                let dist = (date - self.quarter_end(year, q)).num_days().abs();
                if dist < best.2 {
                    best = (year, q, dist);
                }
            }
        }
        best
    }
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map_or(28, |d| d.day())
}

/// Which fiscal periods a fact can supply a value for, derived from its own
/// dates — never from the filing's `fy`, which labels every comparative period
/// in a 10-K with the filing year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactPeriods {
    pub annual: Option<i32>,
    pub quarter: Option<(i32, Quarter)>,
    /// Days between the fact's end date and the nominal period end.
    pub distance_days: i64,
    /// Whether this fact establishes that its annual / quarterly period exists
    /// (flow data, or a fiscal-year-end fact), as opposed to only filling a
    /// period that does. Balance-sheet instants never create quarters, so a
    /// prior year-end balance in a 10-Q can't add a Q4 with no income data.
    pub creates_annual: bool,
    pub creates_quarter: bool,
}

/// Annual durations run 52-53 weeks; allow slack for odd first/last years.
const ANNUAL_DAYS: std::ops::RangeInclusive<i64> = 340..=385;
/// Quarters run 13-14 weeks.
const QUARTER_DAYS: std::ops::RangeInclusive<i64> = 75..=105;
/// Duration facts end on the period end, give or take 52/53-week drift.
const DURATION_MAX_DISTANCE_DAYS: i64 = 16;
/// Instants can be cover-page dates weeks after the period; cap at half a quarter.
const INSTANT_MAX_DISTANCE_DAYS: i64 = 46;

pub fn fact_periods(fact: &FactValue, fye: &FiscalYearEnd) -> Option<FactPeriods> {
    let end = fact.end_date()?;
    let (year, q, distance_days) = fye.nearest_quarter_end(end);
    let duration = fact.duration_days();
    let max_distance = if duration.is_some() {
        DURATION_MAX_DISTANCE_DAYS
    } else {
        INSTANT_MAX_DISTANCE_DAYS
    };
    if distance_days > max_distance {
        return None;
    }

    match duration {
        // A full-year duration counts only when it ends at the fiscal year end
        // (skips trailing-twelve-month figures and pre-FYE-change years).
        Some(d) if ANNUAL_DAYS.contains(&d) => (q == Quarter::Q4).then_some(FactPeriods {
            annual: Some(year),
            quarter: None,
            distance_days,
            creates_annual: true,
            creates_quarter: false,
        }),
        Some(d) if QUARTER_DAYS.contains(&d) => Some(FactPeriods {
            annual: None,
            quarter: Some((year, q)),
            distance_days,
            creates_annual: false,
            creates_quarter: true,
        }),
        // Year-to-date (6/9-month) and other durations don't map to one period.
        Some(_) => None,
        // Instants (balance sheet, cover page) — or a source that omits `start`.
        None => Some(FactPeriods {
            annual: (q == Quarter::Q4).then_some(year),
            quarter: Some((year, q)),
            distance_days,
            creates_annual: q == Quarter::Q4 && fact.fiscal_period.as_deref() == Some("FY"),
            creates_quarter: false,
        }),
    }
}

/// Detect a company's fiscal year-end month from its metadata.
pub fn detect_fiscal_year_end(company: &Company) -> u32 {
    // EDGAR stores fiscal_year_end as MMDD (e.g., "0930" for September 30)
    company
        .fiscal_year_end
        .as_ref()
        .and_then(|fye| {
            if fye.len() >= 2 {
                fye[..2].parse::<u32>().ok()
            } else {
                None
            }
        })
        .unwrap_or(12) // Default to calendar year
}

/// Determine the fiscal period for a fact based on its dates and the fiscal year-end.
pub fn classify_period(fact: &FactValue, fy_end_month: u32) -> Option<FiscalPeriod> {
    // If EDGAR already provides fiscal year and period, use those
    if let (Some(fy), Some(fp)) = (fact.fiscal_year, fact.fiscal_period.as_deref()) {
        let quarter = match fp {
            "Q1" => Some(Quarter::Q1),
            "Q2" => Some(Quarter::Q2),
            "Q3" => Some(Quarter::Q3),
            "Q4" | "FY" => None, // FY = annual
            _ => return None,
        };

        return Some(if fp == "FY" {
            FiscalPeriod::annual(fy)
        } else if let Some(q) = quarter {
            FiscalPeriod::quarterly(fy, q)
        } else {
            FiscalPeriod::annual(fy)
        });
    }

    // Fall back to date-based classification
    let end_date = fact.end_date()?;
    let duration_days = fact.duration_days();

    let fiscal_year = if end_date.month() as u32 <= fy_end_month {
        end_date.year()
    } else {
        end_date.year() + 1
    };

    match duration_days {
        Some(d) if (350..=380).contains(&d) => Some(FiscalPeriod::annual(fiscal_year)),
        Some(d) if (80..=100).contains(&d) => {
            let quarter = quarter_from_end_month(end_date.month(), fy_end_month);
            Some(FiscalPeriod::quarterly(fiscal_year, quarter))
        }
        None => {
            // Instant fact (balance sheet) — classify as annual at fiscal year-end
            Some(FiscalPeriod::annual(fiscal_year))
        }
        _ => None,
    }
}

fn quarter_from_end_month(end_month: u32, fy_end_month: u32) -> Quarter {
    // Calculate which quarter based on fiscal year end
    let months_after_fye = if end_month > fy_end_month {
        end_month - fy_end_month
    } else {
        end_month + 12 - fy_end_month
    };

    match months_after_fye {
        1..=3 => Quarter::Q1,
        4..=6 => Quarter::Q2,
        7..=9 => Quarter::Q3,
        _ => Quarter::Q4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn quarter_ends_follow_fiscal_year_end() {
        let fye = FiscalYearEnd::parse("0930").unwrap();
        assert_eq!(fye.quarter_end(2025, Quarter::Q4), d("2025-09-30"));
        assert_eq!(fye.quarter_end(2025, Quarter::Q1), d("2024-12-30"));
        let jan = FiscalYearEnd::parse("0131").unwrap();
        assert_eq!(jan.quarter_end(2026, Quarter::Q3), d("2025-10-31"));
        assert_eq!(FiscalYearEnd::parse("1231").unwrap().quarter_end(2025, Quarter::Q2), d("2025-06-30"));
    }

    #[test]
    fn nearest_quarter_end_handles_week_based_years() {
        // Apple FY2025 ended 2025-09-27; Q1 FY2026 ended 2025-12-27.
        let fye = FiscalYearEnd::parse("0927").unwrap();
        assert_eq!(fye.nearest_quarter_end(d("2025-09-27")), (2025, Quarter::Q4, 0));
        assert_eq!(fye.nearest_quarter_end(d("2025-12-27")).0, 2026);
        assert_eq!(fye.nearest_quarter_end(d("2025-12-27")).1, Quarter::Q1);
        // A 53-week year ending a few days into the next month.
        let dec = FiscalYearEnd::default();
        assert_eq!(dec.nearest_quarter_end(d("2026-01-02")), (2025, Quarter::Q4, 2));
    }

    #[test]
    fn durations_must_end_near_a_period_end() {
        let fye = FiscalYearEnd::default();
        let fact = |start: &str, end: &str| FactValue {
            filed: "2017-02-28".into(),
            start: Some(start.into()),
            end: end.into(),
            val: Some(1.0),
            accession: "x".into(),
            form: Some("10-K".into()),
            fiscal_year: Some(2016),
            fiscal_period: Some("FY".into()),
            frame: None,
        };
        // A Dec-FYE company's 12 months to Jan 31 is not its fiscal year.
        assert_eq!(fact_periods(&fact("2016-02-01", "2017-01-31"), &fye), None);
        let fy = fact_periods(&fact("2016-01-01", "2016-12-31"), &fye).unwrap();
        assert_eq!(fy.annual, Some(2016));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(FiscalYearEnd::parse("13"), None);
        assert_eq!(FiscalYearEnd::parse("1301"), None);
        assert_eq!(FiscalYearEnd::parse("abcd"), None);
    }
}
