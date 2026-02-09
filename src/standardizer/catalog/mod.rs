pub mod balance_sheet;
pub mod cash_flow;
pub mod income_statement;
pub mod per_share;
pub mod ratios;
pub mod sector;

use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::MetricResolution;

/// A single metric definition with its resolution strategy.
#[derive(Debug, Clone)]
pub struct MetricDefinition {
    pub metric: StandardMetric,
    pub resolution: MetricResolution,
}

/// Trait for metric catalogs. Implement this to provide custom metrics.
pub trait MetricCatalog: Send + Sync {
    fn definitions(&self) -> Vec<MetricDefinition>;
}

/// A catalog backed by a pre-built Vec of definitions.
pub struct VecCatalog(pub Vec<MetricDefinition>);

impl MetricCatalog for VecCatalog {
    fn definitions(&self) -> Vec<MetricDefinition> {
        self.0.clone()
    }
}

/// The default catalog combining all standard metric definitions.
pub struct DefaultCatalog;

impl MetricCatalog for DefaultCatalog {
    fn definitions(&self) -> Vec<MetricDefinition> {
        let mut defs = Vec::new();
        defs.extend(income_statement::definitions());
        defs.extend(balance_sheet::definitions());
        defs.extend(cash_flow::definitions());
        defs.extend(per_share::definitions());
        defs.extend(ratios::definitions());
        defs
    }
}
