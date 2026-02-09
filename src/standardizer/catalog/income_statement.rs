use super::MetricDefinition;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::{MetricResolution, TagSpec};

/// Income statement metric definitions with XBRL tag fallback chains.
///
/// Tag ordering is based on real-world usage across 42 major companies.
pub fn definitions() -> Vec<MetricDefinition> {
    vec![
        // ─── Revenue (36/42 use Revenues, 31/42 use RevenueFromContract...) ───
        MetricDefinition {
            metric: StandardMetric::Revenue,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("RevenueFromContractWithCustomerExcludingAssessedTax"),  // 31/42
                TagSpec::gaap_usd("RevenueFromContractWithCustomerIncludingAssessedTax"),
                TagSpec::gaap_usd("Revenues"),                                             // 36/42
                TagSpec::gaap_usd("SalesRevenueNet"),                                      // 25/42
                TagSpec::gaap_usd("SalesRevenueGoodsNet"),                                 // 21/42
                TagSpec::gaap_usd("SalesRevenueServicesNet"),
                TagSpec::gaap_usd("RegulatedAndUnregulatedOperatingRevenue"),
                TagSpec::gaap_usd("ElectricUtilityRevenue"),
                TagSpec::gaap_usd("RealEstateRevenueNet"),
                TagSpec::gaap_usd("InterestAndDividendIncomeOperating"),
                TagSpec::gaap_usd("HealthCareOrganizationRevenue"),
                TagSpec::gaap_usd("OilAndGasRevenue"),
                TagSpec::gaap_usd("FinancialServicesRevenue"),
            ]),
        },
        // ─── Cost of Revenue (29/42 COGS, 20/42 CostOfRevenue) ───
        MetricDefinition {
            metric: StandardMetric::CostOfRevenue,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("CostOfGoodsAndServicesSold"),                           // 29/42
                TagSpec::gaap_usd("CostOfGoodsSold"),                                      // 23/42
                TagSpec::gaap_usd("CostOfRevenue"),                                        // 20/42
                TagSpec::gaap_usd("CostOfServices"),                                       // 12/42
                TagSpec::gaap_usd("CostOfGoodsAndServiceExcludingDepreciationDepletionAndAmortization"),
            ]),
        },
        // ─── Gross Profit (31/42) ───
        MetricDefinition {
            metric: StandardMetric::GrossProfit,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("GrossProfit"),                                          // 31/42
            ]),
        },
        // ─── R&D (34/42 use ResearchAndDevelopmentExpense) ───
        // NOTE: Amazon (~$100B+ "Technology and content") doesn't tag R&D
        // separately in the EDGAR XBRL API. No extension tags are exposed.
        MetricDefinition {
            metric: StandardMetric::ResearchAndDevelopment,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("ResearchAndDevelopmentExpense"),                         // 34/42
                TagSpec::gaap_usd("ResearchAndDevelopmentExpenseExcludingAcquiredInProcessCost"), // 7/42
                TagSpec::gaap_usd("ResearchAndDevelopmentExpenseSoftwareExcludingAcquiredInProcessCost"), // 3/42
                TagSpec::gaap_usd("OtherResearchAndDevelopmentExpense"),                    // 1/42
            ]),
        },
        // ─── SG&A (29/42 use combined, 13/42 G&A alone, 11/42 S&M alone) ───
        MetricDefinition {
            metric: StandardMetric::SellingGeneralAdmin,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("SellingGeneralAndAdministrativeExpense"),                // 29/42
                TagSpec::gaap_usd("GeneralAndAdministrativeExpense"),                       // 13/42
                TagSpec::gaap_usd("SellingAndMarketingExpense"),                            // 11/42
                TagSpec::gaap_usd("OtherSellingGeneralAndAdministrativeExpense"),           // 1/42
            ]),
        },
        // ─── Operating Expenses ───
        MetricDefinition {
            metric: StandardMetric::OperatingExpenses,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("OperatingExpenses"),
                TagSpec::gaap_usd("CostsAndExpenses"),
            ]),
        },
        // ─── Operating Income (37/42) ───
        MetricDefinition {
            metric: StandardMetric::OperatingIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("OperatingIncomeLoss"),                                  // 37/42
                TagSpec::gaap_usd("IncomeLossFromContinuingOperationsBeforeIncomeTaxesMinorityInterestAndIncomeLossFromEquityMethodInvestments"),
            ]),
        },
        // ─── Interest Expense (35/42) ───
        MetricDefinition {
            metric: StandardMetric::InterestExpense,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("InterestExpense"),                                      // 35/42
                TagSpec::gaap_usd("InterestExpenseDebt"),                                  // 12/42
                TagSpec::gaap_usd("InterestExpenseNonoperating"),                           // 21/42
                TagSpec::gaap_usd("InterestAndDebtExpense"),
                TagSpec::gaap_usd("InterestExpenseLongTermDebt"),                          // 2/42
                TagSpec::gaap_usd("InterestPaidNet"),
            ]),
        },
        // ─── Interest Income ───
        MetricDefinition {
            metric: StandardMetric::InterestIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("InterestIncomeExpenseNet"),
                TagSpec::gaap_usd("InvestmentIncomeInterest"),
                TagSpec::gaap_usd("InterestIncomeOther"),
                TagSpec::gaap_usd("InvestmentIncomeInterestAndDividend"),
            ]),
        },
        // ─── Other Non-Operating Income (36/42) ───
        MetricDefinition {
            metric: StandardMetric::OtherNonOperatingIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("OtherNonoperatingIncomeExpense"),                       // 36/42
                TagSpec::gaap_usd("NonoperatingIncomeExpense"),                            // 22/42
                TagSpec::gaap_usd("OtherNonoperatingIncome"),                              // 7/42
                TagSpec::gaap_usd("OtherOperatingIncomeExpenseNet"),                       // 8/42
                TagSpec::gaap_usd("OtherIncome"),
            ]),
        },
        // ─── Pre-Tax Income ───
        MetricDefinition {
            metric: StandardMetric::PretaxIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest"),
                TagSpec::gaap_usd("IncomeLossFromContinuingOperationsBeforeIncomeTaxesMinorityInterestAndIncomeLossFromEquityMethodInvestments"),
                TagSpec::gaap_usd("IncomeLossFromContinuingOperationsBeforeIncomeTaxesDomestic"),
                TagSpec::gaap_usd("IncomeLossFromContinuingOperationsBeforeIncomeTaxesForeign"),
            ]),
        },
        // ─── Income Tax Expense ───
        MetricDefinition {
            metric: StandardMetric::IncomeTaxExpense,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("IncomeTaxExpenseBenefit"),
                TagSpec::gaap_usd("CurrentIncomeTaxExpenseBenefit"),
            ]),
        },
        // ─── Net Income (42/42 — universal!) ───
        MetricDefinition {
            metric: StandardMetric::NetIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("NetIncomeLoss"),                                        // 42/42
                TagSpec::gaap_usd("ProfitLoss"),                                           // 29/42
                TagSpec::gaap_usd("NetIncomeLossAvailableToCommonStockholdersBasic"),       // 22/42
                TagSpec::gaap_usd("IncomeLossFromContinuingOperations"),
            ]),
        },
        // ─── Net Income to Common (22/42) ───
        MetricDefinition {
            metric: StandardMetric::NetIncomeToCommon,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("NetIncomeLossAvailableToCommonStockholdersBasic"),       // 22/42
                TagSpec::gaap_usd("NetIncomeLossAvailableToCommonStockholdersDiluted"),     // 17/42
                TagSpec::gaap_usd("NetIncomeLoss"),                                        // 42/42
            ]),
        },
        // ─── D&A (32/42 DDA, 17/42 DA, 33/42 Depreciation alone) ───
        MetricDefinition {
            metric: StandardMetric::DepreciationAmortization,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("DepreciationDepletionAndAmortization"),                 // 32/42
                TagSpec::gaap_usd("DepreciationAndAmortization"),                          // 17/42
                TagSpec::gaap_usd("Depreciation"),                                         // 33/42
                TagSpec::gaap_usd("DepreciationAmortizationAndAccretionNet"),              // 5/42
                TagSpec::gaap_usd("OtherDepreciationAndAmortization"),                    // 4/42
            ]),
        },
    ]
}
