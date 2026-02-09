use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::EdgarError;

/// Fiscal quarter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Quarter {
    Q1,
    Q2,
    Q3,
    Q4,
}

impl fmt::Display for Quarter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Q1 => write!(f, "Q1"),
            Self::Q2 => write!(f, "Q2"),
            Self::Q3 => write!(f, "Q3"),
            Self::Q4 => write!(f, "Q4"),
        }
    }
}

/// A fiscal period combining year and optional quarter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FiscalPeriod {
    pub year: i32,
    pub quarter: Option<Quarter>,
}

impl FiscalPeriod {
    pub fn annual(year: i32) -> Self {
        Self {
            year,
            quarter: None,
        }
    }

    pub fn quarterly(year: i32, quarter: Quarter) -> Self {
        Self {
            year,
            quarter: Some(quarter),
        }
    }
}

impl fmt::Display for FiscalPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.quarter {
            Some(q) => write!(f, "FY{} {}", self.year, q),
            None => write!(f, "FY{}", self.year),
        }
    }
}

/// Calendar period used in EDGAR frames API: `CY2023`, `CY2023Q1`, `CY2023Q1I` (instantaneous).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CalendarPeriod {
    pub year: i32,
    pub quarter: Option<Quarter>,
    pub instantaneous: bool,
}

impl CalendarPeriod {
    pub fn annual(year: i32) -> Self {
        Self {
            year,
            quarter: None,
            instantaneous: false,
        }
    }

    pub fn quarterly(year: i32, quarter: Quarter) -> Self {
        Self {
            year,
            quarter: Some(quarter),
            instantaneous: false,
        }
    }

    pub fn instantaneous(mut self) -> Self {
        self.instantaneous = true;
        self
    }
}

impl fmt::Display for CalendarPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CY{}", self.year)?;
        if let Some(q) = &self.quarter {
            write!(f, "{q}")?;
        }
        if self.instantaneous {
            write!(f, "I")?;
        }
        Ok(())
    }
}

impl FromStr for CalendarPeriod {
    type Err = EdgarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let make_err = || EdgarError::InvalidPeriod(s.to_string());

        if !s.starts_with("CY") {
            return Err(make_err());
        }
        let rest = &s[2..];

        let instantaneous = rest.ends_with('I');
        let rest = if instantaneous {
            &rest[..rest.len() - 1]
        } else {
            rest
        };

        let (year_str, quarter) = if rest.contains('Q') {
            let parts: Vec<&str> = rest.splitn(2, 'Q').collect();
            let q = match parts[1] {
                "1" => Quarter::Q1,
                "2" => Quarter::Q2,
                "3" => Quarter::Q3,
                "4" => Quarter::Q4,
                _ => return Err(make_err()),
            };
            (parts[0], Some(q))
        } else {
            (rest, None)
        };

        let year: i32 = year_str.parse().map_err(|_| make_err())?;

        Ok(Self {
            year,
            quarter,
            instantaneous,
        })
    }
}

/// Duration or instant period from XBRL facts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactPeriod {
    /// A point-in-time value (balance sheet items).
    Instant {
        date: chrono::NaiveDate,
    },
    /// A duration value (income statement, cash flow items).
    Duration {
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    },
}

impl FactPeriod {
    pub fn end_date(&self) -> chrono::NaiveDate {
        match self {
            Self::Instant { date } => *date,
            Self::Duration { end, .. } => *end,
        }
    }

    /// Duration in days. Instants return 0.
    pub fn duration_days(&self) -> i64 {
        match self {
            Self::Instant { .. } => 0,
            Self::Duration { start, end } => (*end - *start).num_days(),
        }
    }

    /// Returns true if the duration is roughly a quarter (80-100 days).
    pub fn is_quarterly(&self) -> bool {
        let days = self.duration_days();
        (80..=100).contains(&days)
    }

    /// Returns true if the duration is roughly annual (350-380 days).
    pub fn is_annual(&self) -> bool {
        let days = self.duration_days();
        (350..=380).contains(&days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_period_roundtrip() {
        let cases = vec![
            ("CY2023", CalendarPeriod::annual(2023)),
            ("CY2023Q1", CalendarPeriod::quarterly(2023, Quarter::Q1)),
            (
                "CY2023Q4I",
                CalendarPeriod::quarterly(2023, Quarter::Q4).instantaneous(),
            ),
        ];
        for (s, expected) in cases {
            let parsed: CalendarPeriod = s.parse().unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.to_string(), s);
        }
    }
}
