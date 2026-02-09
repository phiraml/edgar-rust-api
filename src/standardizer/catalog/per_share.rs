use super::MetricDefinition;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::{MetricResolution, TagSpec};

/// Per-share metric definitions with XBRL tag fallback chains.
///
/// Tag ordering is based on real-world usage across 42 major companies.
pub fn definitions() -> Vec<MetricDefinition> {
    vec![
        // ─── EPS Basic (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::EarningsPerShareBasic,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_per_share("EarningsPerShareBasic"),                         // 42/42
                TagSpec::gaap_per_share("EarningsPerShareBasicAndDiluted"),               // 4/42
                TagSpec::gaap_per_share("IncomeLossFromContinuingOperationsPerBasicShare"),
            ]),
        },
        // ─── EPS Diluted (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::EarningsPerShareDiluted,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_per_share("EarningsPerShareDiluted"),                       // 42/42
                TagSpec::gaap_per_share("EarningsPerShareBasicAndDiluted"),               // 4/42
                TagSpec::gaap_per_share("IncomeLossFromContinuingOperationsPerDilutedShare"),
            ]),
        },
        // ─── Dividends Per Share (32/42 use CommonStockDividendsPerShareDeclared) ───
        MetricDefinition {
            metric: StandardMetric::DividendsPerShare,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_per_share("CommonStockDividendsPerShareDeclared"),           // 32/42
                TagSpec::gaap_per_share("CommonStockDividendsPerShareCashPaid"),           // 21/42
            ]),
        },
        // ─── Shares Outstanding Basic (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::SharesOutstandingBasic,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_shares("WeightedAverageNumberOfSharesOutstandingBasic"),    // 42/42
                TagSpec::gaap_shares("CommonStockSharesOutstanding"),                     // 29/42
                TagSpec::gaap_shares("WeightedAverageNumberOfShareOutstandingBasicAndDiluted"), // 6/42
                TagSpec::gaap_shares("SharesOutstanding"),                                // 2/42
                TagSpec::dei("EntityCommonStockSharesOutstanding"),
            ]),
        },
        // ─── Shares Outstanding Diluted (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::SharesOutstandingDiluted,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_shares("WeightedAverageNumberOfDilutedSharesOutstanding"),  // 42/42
                TagSpec::gaap_shares("WeightedAverageNumberOfShareOutstandingBasicAndDiluted"), // 6/42
            ]),
        },
    ]
}
