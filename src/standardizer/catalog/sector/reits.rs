use crate::standardizer::catalog::MetricDefinition;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::{MetricResolution, TagSpec};

pub fn definitions() -> Vec<MetricDefinition> {
    vec![
        MetricDefinition {
            metric: StandardMetric::FundsFromOperations,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("FundsFromOperations"),
                TagSpec::gaap_usd("FFOPerShareDiluted"),
            ]),
        },
        MetricDefinition {
            metric: StandardMetric::AdjustedFundsFromOperations,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("AdjustedFundsFromOperations"),
            ]),
        },
        MetricDefinition {
            metric: StandardMetric::NetOperatingIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("RealEstateRevenueNet"),
                TagSpec::gaap_usd("OperatingIncomeLoss"),
            ]),
        },
    ]
}
