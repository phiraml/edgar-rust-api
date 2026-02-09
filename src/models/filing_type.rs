use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::EnumString;

/// Common SEC filing types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, Serialize, Deserialize)]
pub enum FilingType {
    #[strum(serialize = "10-K")]
    #[serde(rename = "10-K")]
    TenK,

    #[strum(serialize = "10-K/A")]
    #[serde(rename = "10-K/A")]
    TenKA,

    #[strum(serialize = "10-Q")]
    #[serde(rename = "10-Q")]
    TenQ,

    #[strum(serialize = "10-Q/A")]
    #[serde(rename = "10-Q/A")]
    TenQA,

    #[strum(serialize = "8-K")]
    #[serde(rename = "8-K")]
    EightK,

    #[strum(serialize = "8-K/A")]
    #[serde(rename = "8-K/A")]
    EightKA,

    #[strum(serialize = "20-F")]
    #[serde(rename = "20-F")]
    TwentyF,

    #[strum(serialize = "20-F/A")]
    #[serde(rename = "20-F/A")]
    TwentyFA,

    #[strum(serialize = "40-F")]
    #[serde(rename = "40-F")]
    FortyF,

    #[strum(serialize = "6-K")]
    #[serde(rename = "6-K")]
    SixK,

    #[strum(serialize = "S-1")]
    #[serde(rename = "S-1")]
    S1,

    #[strum(serialize = "S-1/A")]
    #[serde(rename = "S-1/A")]
    S1A,

    #[strum(serialize = "DEF 14A")]
    #[serde(rename = "DEF 14A")]
    Def14A,

    #[strum(serialize = "SC 13D")]
    #[serde(rename = "SC 13D")]
    Sc13D,

    #[strum(serialize = "SC 13G")]
    #[serde(rename = "SC 13G")]
    Sc13G,

    #[strum(serialize = "4")]
    #[serde(rename = "4")]
    Form4,

    #[strum(serialize = "3")]
    #[serde(rename = "3")]
    Form3,

    #[strum(serialize = "5")]
    #[serde(rename = "5")]
    Form5,

    #[strum(disabled)]
    Other(String),
}

impl FilingType {
    pub fn is_annual(&self) -> bool {
        matches!(self, Self::TenK | Self::TenKA | Self::TwentyF | Self::TwentyFA | Self::FortyF)
    }

    pub fn is_quarterly(&self) -> bool {
        matches!(self, Self::TenQ | Self::TenQA)
    }

    pub fn is_periodic(&self) -> bool {
        self.is_annual() || self.is_quarterly()
    }

    pub fn is_amendment(&self) -> bool {
        matches!(
            self,
            Self::TenKA | Self::TenQA | Self::EightKA | Self::TwentyFA | Self::S1A
        )
    }

    /// Parse a filing type string, falling back to `Other` for unknown types.
    pub fn parse_lenient(s: &str) -> Self {
        Self::from_str(s).unwrap_or_else(|_| Self::Other(s.to_string()))
    }
}

impl fmt::Display for FilingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TenK => write!(f, "10-K"),
            Self::TenKA => write!(f, "10-K/A"),
            Self::TenQ => write!(f, "10-Q"),
            Self::TenQA => write!(f, "10-Q/A"),
            Self::EightK => write!(f, "8-K"),
            Self::EightKA => write!(f, "8-K/A"),
            Self::TwentyF => write!(f, "20-F"),
            Self::TwentyFA => write!(f, "20-F/A"),
            Self::FortyF => write!(f, "40-F"),
            Self::SixK => write!(f, "6-K"),
            Self::S1 => write!(f, "S-1"),
            Self::S1A => write!(f, "S-1/A"),
            Self::Def14A => write!(f, "DEF 14A"),
            Self::Sc13D => write!(f, "SC 13D"),
            Self::Sc13G => write!(f, "SC 13G"),
            Self::Form4 => write!(f, "4"),
            Self::Form3 => write!(f, "3"),
            Self::Form5 => write!(f, "5"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known() {
        assert_eq!(FilingType::from_str("10-K").unwrap(), FilingType::TenK);
        assert_eq!(FilingType::from_str("8-K").unwrap(), FilingType::EightK);
    }

    #[test]
    fn parse_lenient_unknown() {
        let ft = FilingType::parse_lenient("NPORT-P");
        assert_eq!(ft, FilingType::Other("NPORT-P".to_string()));
    }

    #[test]
    fn annual_quarterly() {
        assert!(FilingType::TenK.is_annual());
        assert!(FilingType::TenQ.is_quarterly());
        assert!(!FilingType::EightK.is_periodic());
    }

    #[test]
    fn display_roundtrip() {
        assert_eq!(FilingType::TenK.to_string(), "10-K");
        assert_eq!(
            FilingType::from_str(&FilingType::TenK.to_string()).unwrap(),
            FilingType::TenK
        );
    }
}
