use crate::standardizer::catalog::MetricDefinition;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::{MetricResolution, TagSpec};

pub fn definitions() -> Vec<MetricDefinition> {
    vec![
        MetricDefinition {
            metric: StandardMetric::PremiumsEarned,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("PremiumsEarnedNet"),
                TagSpec::gaap_usd("PremiumsWrittenNet"),
                TagSpec::gaap_usd("InsurancePremiumsRevenueRecognized"),
            ]),
        },
    ]
}
