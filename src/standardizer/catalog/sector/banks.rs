use crate::standardizer::catalog::MetricDefinition;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::{MetricResolution, TagSpec};

pub fn definitions() -> Vec<MetricDefinition> {
    vec![
        MetricDefinition {
            metric: StandardMetric::NetInterestIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("InterestIncomeExpenseNet"),
                TagSpec::gaap_usd("NetInterestIncome"),
                TagSpec::gaap_usd("InterestIncomeExpenseAfterProvisionForLoanLoss"),
            ]),
        },
        MetricDefinition {
            metric: StandardMetric::ProvisionForCreditLosses,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("ProvisionForLoanLeaseAndOtherLosses"),
                TagSpec::gaap_usd("ProvisionForLoanAndLeaseLosses"),
                TagSpec::gaap_usd("ProvisionForCreditLosses"),
            ]),
        },
        MetricDefinition {
            metric: StandardMetric::NonInterestIncome,
            resolution: MetricResolution::FirstMatch(vec![
                TagSpec::gaap_usd("NoninterestIncome"),
                TagSpec::gaap_usd("RevenueFromContractWithCustomerExcludingAssessedTax"),
            ]),
        },
    ]
}
