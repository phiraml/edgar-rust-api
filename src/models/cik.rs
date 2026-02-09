use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::EdgarError;

/// A SEC Central Index Key, always stored as a `u64` and displayed zero-padded to 10 digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cik(u64);

impl Cik {
    /// Create a new CIK from a numeric value.
    pub fn new(value: u64) -> crate::error::Result<Self> {
        if value == 0 || value > 9_999_999_999 {
            return Err(EdgarError::InvalidCik(value.to_string()));
        }
        Ok(Self(value))
    }

    /// Return the raw numeric value.
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Return the 10-digit zero-padded string used in EDGAR URLs.
    pub fn zero_padded(self) -> String {
        format!("{:010}", self.0)
    }
}

impl fmt::Display for Cik {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:010}", self.0)
    }
}

impl FromStr for Cik {
    type Err = EdgarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim().trim_start_matches("CIK").trim_start_matches('0');
        let value: u64 = trimmed
            .parse()
            .map_err(|_| EdgarError::InvalidCik(s.to_string()))?;
        Self::new(value)
    }
}

impl From<u64> for Cik {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

impl Serialize for Cik {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for Cik {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CikVisitor;

        impl serde::de::Visitor<'_> for CikVisitor {
            type Value = Cik;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a CIK as integer or string")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Cik, E> {
                Ok(Cik(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Cik, E> {
                Ok(Cik(v as u64))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Cik, E> {
                v.parse().map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_any(CikVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_padded_format() {
        let cik = Cik::new(320193).unwrap();
        assert_eq!(cik.zero_padded(), "0000320193");
        assert_eq!(cik.to_string(), "0000320193");
    }

    #[test]
    fn parse_from_string() {
        assert_eq!(Cik::from_str("320193").unwrap().as_u64(), 320193);
        assert_eq!(Cik::from_str("CIK0000320193").unwrap().as_u64(), 320193);
        assert_eq!(Cik::from_str("0000320193").unwrap().as_u64(), 320193);
    }

    #[test]
    fn invalid_cik() {
        assert!(Cik::new(0).is_err());
        assert!(Cik::from_str("abc").is_err());
    }
}
