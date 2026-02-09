use chrono::Datelike;

use crate::models::company::Company;
use crate::models::company_facts::FactValue;
use crate::models::period::{FiscalPeriod, Quarter};

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
