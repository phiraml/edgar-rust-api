use super::MetricDefinition;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::{MetricResolution, TagSpec};

/// Cash flow statement metric definitions with XBRL tag fallback chains.
///
/// Tag ordering is based on real-world usage across 42 major companies.
pub fn definitions() -> Vec<MetricDefinition> {
    vec![
        // ─── Operating Cash Flow (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::OperatingCashFlow,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("NetCashProvidedByUsedInOperatingActivities"),           // 42/42
                TagSpec::gaap_usd("NetCashProvidedByUsedInOperatingActivitiesContinuingOperations"), // 20/42
            ]),
        },
        // ─── Capital Expenditures (38/42 use PaymentsToAcquirePropertyPlantAndEquipment) ───
        MetricDefinition {
            metric: StandardMetric::CapitalExpenditures,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("PaymentsToAcquirePropertyPlantAndEquipment"),          // 38/42
                TagSpec::gaap_usd("PaymentsToAcquireProductiveAssets"),
                TagSpec::gaap_usd("CapitalExpendituresIncurredButNotYetPaid"),             // 15/42
            ]),
        },
        // ─── Investing Cash Flow (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::InvestingCashFlow,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("NetCashProvidedByUsedInInvestingActivities"),           // 42/42
                TagSpec::gaap_usd("NetCashProvidedByUsedInInvestingActivitiesContinuingOperations"), // 20/42
            ]),
        },
        // ─── Financing Cash Flow (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::FinancingCashFlow,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("NetCashProvidedByUsedInFinancingActivities"),           // 42/42
                TagSpec::gaap_usd("NetCashProvidedByUsedInFinancingActivitiesContinuingOperations"), // 19/42
            ]),
        },
        // ─── Dividends Paid (21/42 use PaymentsOfDividends) ───
        MetricDefinition {
            metric: StandardMetric::DividendsPaid,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("PaymentsOfDividendsCommonStock"),                      // 19/42
                TagSpec::gaap_usd("PaymentsOfDividends"),                                 // 21/42
                TagSpec::gaap_usd("PaymentsOfOrdinaryDividends"),
                TagSpec::gaap_usd("DividendsCommonStockCash"),                            // 23/42
                TagSpec::gaap_usd("Dividends"),                                           // 8/42
                TagSpec::gaap_usd("DividendsCash"),                                       // 6/42
            ]),
        },
        // ─── Share Repurchases (41/42 use PaymentsForRepurchaseOfCommonStock) ───
        MetricDefinition {
            metric: StandardMetric::ShareRepurchases,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("PaymentsForRepurchaseOfCommonStock"),                  // 41/42
                TagSpec::gaap_usd("PaymentsForRepurchaseOfEquity"),                       // 4/42
            ]),
        },
        // ─── Net Change in Cash (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::NetChangeInCash,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalentsPeriodIncreaseDecreaseIncludingExchangeRateEffect"), // 42/42
                TagSpec::gaap_usd("CashAndCashEquivalentsPeriodIncreaseDecrease"),        // 41/42
                TagSpec::gaap_usd("CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalentsPeriodIncreaseDecreaseExcludingExchangeRateEffect"),
            ]),
        },
    ]
}
