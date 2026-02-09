use std::sync::Arc;

use super::engine::FactIndex;

/// Specifies a single XBRL tag to look up.
#[derive(Debug, Clone)]
pub struct TagSpec {
    /// The XBRL taxonomy (e.g., "us-gaap", "dei", "ifrs-full").
    pub taxonomy: String,
    /// The XBRL tag name.
    pub tag: String,
    /// Preferred unit (e.g., "USD", "shares", "USD/shares"). None = any.
    pub unit: Option<String>,
}

impl TagSpec {
    pub fn gaap(tag: &str) -> Self {
        Self {
            taxonomy: "us-gaap".to_string(),
            tag: tag.to_string(),
            unit: None,
        }
    }

    pub fn gaap_usd(tag: &str) -> Self {
        Self {
            taxonomy: "us-gaap".to_string(),
            tag: tag.to_string(),
            unit: Some("USD".to_string()),
        }
    }

    pub fn gaap_shares(tag: &str) -> Self {
        Self {
            taxonomy: "us-gaap".to_string(),
            tag: tag.to_string(),
            unit: Some("shares".to_string()),
        }
    }

    pub fn gaap_per_share(tag: &str) -> Self {
        Self {
            taxonomy: "us-gaap".to_string(),
            tag: tag.to_string(),
            unit: Some("USD/shares".to_string()),
        }
    }

    pub fn dei(tag: &str) -> Self {
        Self {
            taxonomy: "dei".to_string(),
            tag: tag.to_string(),
            unit: None,
        }
    }
}

/// How to resolve a standardized metric from raw XBRL facts.
#[derive(Clone)]
pub enum MetricResolution {
    /// Try tags in order, take the first one that has data.
    FirstMatch(Vec<TagSpec>),

    /// Sum values from multiple tags.
    Sum(Vec<TagSpec>),

    /// Compute A - B.
    Difference(Box<TagSpec>, Box<TagSpec>),

    /// Compute A / B.
    Ratio(Box<MetricResolution>, Box<MetricResolution>),

    /// Custom resolution function.
    Custom(Arc<dyn Fn(&FactIndex) -> Option<f64> + Send + Sync>),
}

impl MetricResolution {
    /// Extract all TagSpecs referenced by this resolution.
    /// Useful for coverage analysis to know which tags were tried.
    pub fn tag_specs(&self) -> Vec<&TagSpec> {
        match self {
            Self::FirstMatch(specs) | Self::Sum(specs) => specs.iter().collect(),
            Self::Difference(a, b) => vec![a.as_ref(), b.as_ref()],
            Self::Ratio(a, b) => {
                let mut out = a.tag_specs();
                out.extend(b.tag_specs());
                out
            }
            Self::Custom(_) => vec![],
        }
    }
}

impl std::fmt::Debug for MetricResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstMatch(tags) => f.debug_tuple("FirstMatch").field(tags).finish(),
            Self::Sum(tags) => f.debug_tuple("Sum").field(tags).finish(),
            Self::Difference(a, b) => f.debug_tuple("Difference").field(a).field(b).finish(),
            Self::Ratio(a, b) => f.debug_tuple("Ratio").field(a).field(b).finish(),
            Self::Custom(_) => f.debug_tuple("Custom").field(&"<fn>").finish(),
        }
    }
}
