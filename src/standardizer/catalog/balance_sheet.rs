use super::MetricDefinition;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::{MetricResolution, TagSpec};

/// Balance sheet metric definitions with XBRL tag fallback chains.
///
/// Tag ordering is based on real-world usage across 42 major companies.
pub fn definitions() -> Vec<MetricDefinition> {
    vec![
        // ─── Cash & Equivalents (42/42 use CashAndCashEquivalentsAtCarryingValue) ───
        MetricDefinition {
            metric: StandardMetric::CashAndEquivalents,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("CashAndCashEquivalentsAtCarryingValue"),              // 42/42
                TagSpec::gaap_usd("CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalents"), // 39/42
                TagSpec::gaap_usd("CashCashEquivalentsAndShortTermInvestments"),          // 14/42
                TagSpec::gaap_usd("Cash"),
                TagSpec::gaap_usd("CashEquivalentsAtCarryingValue"),
            ]),
        },
        // ─── Short-Term Investments ───
        MetricDefinition {
            metric: StandardMetric::ShortTermInvestments,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("ShortTermInvestments"),
                TagSpec::gaap_usd("MarketableSecuritiesCurrent"),
                TagSpec::gaap_usd("AvailableForSaleSecuritiesDebtSecuritiesCurrent"),
            ]),
        },
        // ─── Cash & Short-Term Investments ───
        MetricDefinition {
            metric: StandardMetric::CashAndShortTermInvestments,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("CashCashEquivalentsAndShortTermInvestments"),          // 14/42
                TagSpec::gaap_usd("CashAndCashEquivalentsAtCarryingValue"),               // 42/42 (fallback)
            ]),
        },
        // ─── Accounts Receivable (36/42 use AccountsReceivableNetCurrent) ───
        MetricDefinition {
            metric: StandardMetric::AccountsReceivable,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("AccountsReceivableNetCurrent"),                        // 36/42
                TagSpec::gaap_usd("ReceivablesNetCurrent"),                               // 10/42
                TagSpec::gaap_usd("AccountsReceivableNet"),                               // 3/42
                TagSpec::gaap_usd("AccountsAndOtherReceivablesNetCurrent"),               // 3/42
                TagSpec::gaap_usd("AccountsNotesAndLoansReceivableNetCurrent"),
            ]),
        },
        // ─── Inventory (35/42 use InventoryNet) ───
        MetricDefinition {
            metric: StandardMetric::Inventory,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("InventoryNet"),                                        // 35/42
                TagSpec::gaap_usd("InventoryFinishedGoods"),                              // 20/42
                TagSpec::gaap_usd("InventoryFinishedGoodsNetOfReserves"),                 // 24/42
                TagSpec::gaap_usd("InventoryNetOfAllowancesCustomerAdvancesAndProgressBillings"), // 2/42
                TagSpec::gaap_usd("InventoryGross"),
            ]),
        },
        // ─── Other Current Assets ───
        MetricDefinition {
            metric: StandardMetric::OtherCurrentAssets,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("OtherAssetsCurrent"),
                TagSpec::gaap_usd("PrepaidExpenseAndOtherAssetsCurrent"),
                TagSpec::gaap_usd("PrepaidExpenseCurrent"),
            ]),
        },
        // ─── Total Current Assets (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::TotalCurrentAssets,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("AssetsCurrent"),                                       // 42/42
            ]),
        },
        // ─── PP&E ───
        MetricDefinition {
            metric: StandardMetric::PropertyPlantEquipment,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("PropertyPlantAndEquipmentNet"),
                TagSpec::gaap_usd("PropertyPlantAndEquipmentAndFinanceLeaseRightOfUseAssetAfterAccumulatedDepreciationAndAmortization"), // 9/42
                TagSpec::gaap_usd("PropertyPlantAndEquipmentGross"),
            ]),
        },
        // ─── Goodwill ───
        MetricDefinition {
            metric: StandardMetric::Goodwill,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("Goodwill"),
                TagSpec::gaap_usd("GoodwillAndIntangibleAssetsNet"),
            ]),
        },
        // ─── Intangible Assets ───
        MetricDefinition {
            metric: StandardMetric::IntangibleAssets,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("IntangibleAssetsNetExcludingGoodwill"),
                TagSpec::gaap_usd("FiniteLivedIntangibleAssetsNet"),
                TagSpec::gaap_usd("IndefiniteLivedIntangibleAssetsExcludingGoodwill"),
            ]),
        },
        // ─── Other Non-Current Assets (42/42 use OtherAssetsNoncurrent) ───
        MetricDefinition {
            metric: StandardMetric::OtherNonCurrentAssets,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("OtherAssetsNoncurrent"),                               // 42/42
                TagSpec::gaap_usd("OtherAssets"),
            ]),
        },
        // ─── Total Assets (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::TotalAssets,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("Assets"),                                              // 42/42
            ]),
        },
        // ─── Accounts Payable (40/42 use AccountsPayableCurrent) ───
        MetricDefinition {
            metric: StandardMetric::AccountsPayable,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("AccountsPayableCurrent"),                              // 40/42
                TagSpec::gaap_usd("AccountsPayableAndAccruedLiabilitiesCurrent"),         // 5/42
                TagSpec::gaap_usd("AccountsPayableTradeCurrent"),                         // 4/42
                TagSpec::gaap_usd("AccountsPayableAndOtherAccruedLiabilitiesCurrent"),    // 3/42
                TagSpec::gaap_usd("AccountsPayableCurrentAndNoncurrent"),                 // 2/42
            ]),
        },
        // ─── Short-Term Debt ───
        MetricDefinition {
            metric: StandardMetric::ShortTermDebt,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("ShortTermBorrowings"),
                TagSpec::gaap_usd("CommercialPaper"),
                TagSpec::gaap_usd("ShortTermDebt"),
            ]),
        },
        // ─── Current Portion of Long-Term Debt (30/42 use LongTermDebtCurrent) ───
        MetricDefinition {
            metric: StandardMetric::CurrentPortionLongTermDebt,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("LongTermDebtCurrent"),                                 // 30/42
                TagSpec::gaap_usd("LongTermDebtAndCapitalLeaseObligationsCurrent"),       // 11/42
            ]),
        },
        // ─── Other Current Liabilities ───
        MetricDefinition {
            metric: StandardMetric::OtherCurrentLiabilities,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("OtherLiabilitiesCurrent"),                             // 27/42
                TagSpec::gaap_usd("AccruedLiabilitiesCurrent"),                           // 28/42
                TagSpec::gaap_usd("EmployeeRelatedLiabilitiesCurrent"),                   // 38/42
            ]),
        },
        // ─── Total Current Liabilities (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::TotalCurrentLiabilities,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("LiabilitiesCurrent"),                                  // 42/42
            ]),
        },
        // ─── Long-Term Debt (36/42 use LongTermDebt) ───
        MetricDefinition {
            metric: StandardMetric::LongTermDebt,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("LongTermDebtNoncurrent"),                              // 33/42
                TagSpec::gaap_usd("LongTermDebt"),                                        // 36/42
                TagSpec::gaap_usd("LongTermDebtAndCapitalLeaseObligations"),              // 12/42
                TagSpec::gaap_usd("OtherLongTermDebt"),                                   // 8/42
            ]),
        },
        // ─── Other Non-Current Liabilities (41/42 use OtherLiabilitiesNoncurrent) ───
        MetricDefinition {
            metric: StandardMetric::OtherNonCurrentLiabilities,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("OtherLiabilitiesNoncurrent"),                          // 41/42
                TagSpec::gaap_usd("OtherLiabilities"),
            ]),
        },
        // ─── Total Liabilities (30/42 use Liabilities) ───
        MetricDefinition {
            metric: StandardMetric::TotalLiabilities,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("Liabilities"),                                         // 30/42
                TagSpec::gaap_usd("LiabilitiesAndStockholdersEquity"),                    // 42/42 (includes equity)
            ]),
        },
        // ─── Common Stock ───
        MetricDefinition {
            metric: StandardMetric::CommonStock,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("CommonStockValue"),
                TagSpec::gaap_usd("CommonStocksIncludingAdditionalPaidInCapital"),
            ]),
        },
        // ─── Retained Earnings ───
        MetricDefinition {
            metric: StandardMetric::RetainedEarnings,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("RetainedEarningsAccumulatedDeficit"),
            ]),
        },
        // ─── Accumulated Other Comprehensive Income ───
        MetricDefinition {
            metric: StandardMetric::AccumulatedOtherComprehensiveIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("AccumulatedOtherComprehensiveIncomeLossNetOfTax"),
            ]),
        },
        // ─── Total Stockholders' Equity (39/42 use StockholdersEquity) ───
        MetricDefinition {
            metric: StandardMetric::TotalStockholdersEquity,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("StockholdersEquity"),                                  // 39/42
                TagSpec::gaap_usd("StockholdersEquityIncludingPortionAttributableToNoncontrollingInterest"), // 30/42
            ]),
        },
        // ─── Total Liabilities & Equity (42/42 — universal) ───
        MetricDefinition {
            metric: StandardMetric::TotalLiabilitiesAndEquity,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("LiabilitiesAndStockholdersEquity"),                    // 42/42
            ]),
        },
    ]
}
