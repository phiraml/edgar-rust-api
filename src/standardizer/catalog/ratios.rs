use std::sync::Arc;

use super::MetricDefinition;
use crate::standardizer::engine::FactIndex;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::MetricResolution;

/// Computed ratio definitions.
///
/// Ratios are computed from already-resolved metrics, so they use `Custom` resolution
/// with closures that reference other StandardMetric values. The engine runs these
/// in a second pass after all primary metrics are resolved.
pub fn definitions() -> Vec<MetricDefinition> {
    vec![
        ratio_def(StandardMetric::GrossMargin, |idx| {
            div(idx.resolved(StandardMetric::GrossProfit)?, idx.resolved(StandardMetric::Revenue)?)
        }),
        ratio_def(StandardMetric::OperatingMargin, |idx| {
            div(idx.resolved(StandardMetric::OperatingIncome)?, idx.resolved(StandardMetric::Revenue)?)
        }),
        ratio_def(StandardMetric::NetMargin, |idx| {
            div(idx.resolved(StandardMetric::NetIncome)?, idx.resolved(StandardMetric::Revenue)?)
        }),
        ratio_def(StandardMetric::ReturnOnAssets, |idx| {
            div(idx.resolved(StandardMetric::NetIncome)?, idx.resolved(StandardMetric::TotalAssets)?)
        }),
        ratio_def(StandardMetric::ReturnOnEquity, |idx| {
            div(idx.resolved(StandardMetric::NetIncome)?, idx.resolved(StandardMetric::TotalStockholdersEquity)?)
        }),
        ratio_def(StandardMetric::CurrentRatio, |idx| {
            div(idx.resolved(StandardMetric::TotalCurrentAssets)?, idx.resolved(StandardMetric::TotalCurrentLiabilities)?)
        }),
        ratio_def(StandardMetric::QuickRatio, |idx| {
            let ca = idx.resolved(StandardMetric::TotalCurrentAssets)?;
            let inv = idx.resolved(StandardMetric::Inventory).unwrap_or(0.0);
            let cl = idx.resolved(StandardMetric::TotalCurrentLiabilities)?;
            div(ca - inv, cl)
        }),
        ratio_def(StandardMetric::DebtToEquity, |idx| {
            let ltd = idx.resolved(StandardMetric::LongTermDebt).unwrap_or(0.0);
            let std = idx.resolved(StandardMetric::ShortTermDebt).unwrap_or(0.0);
            let eq = idx.resolved(StandardMetric::TotalStockholdersEquity)?;
            div(ltd + std, eq)
        }),
        ratio_def(StandardMetric::DebtToAssets, |idx| {
            let ltd = idx.resolved(StandardMetric::LongTermDebt).unwrap_or(0.0);
            let std = idx.resolved(StandardMetric::ShortTermDebt).unwrap_or(0.0);
            let ta = idx.resolved(StandardMetric::TotalAssets)?;
            div(ltd + std, ta)
        }),
        ratio_def(StandardMetric::InterestCoverage, |idx| {
            div(idx.resolved(StandardMetric::OperatingIncome)?, idx.resolved(StandardMetric::InterestExpense)?)
        }),
        ratio_def(StandardMetric::AssetTurnover, |idx| {
            div(idx.resolved(StandardMetric::Revenue)?, idx.resolved(StandardMetric::TotalAssets)?)
        }),
        ratio_def(StandardMetric::InventoryTurnover, |idx| {
            div(idx.resolved(StandardMetric::CostOfRevenue)?, idx.resolved(StandardMetric::Inventory)?)
        }),
        ratio_def(StandardMetric::ReceivablesTurnover, |idx| {
            div(idx.resolved(StandardMetric::Revenue)?, idx.resolved(StandardMetric::AccountsReceivable)?)
        }),
        ratio_def(StandardMetric::FreeCashFlowMargin, |idx| {
            let ocf = idx.resolved(StandardMetric::OperatingCashFlow)?;
            let capex = idx.resolved(StandardMetric::CapitalExpenditures).unwrap_or(0.0);
            let rev = idx.resolved(StandardMetric::Revenue)?;
            div(ocf - capex.abs(), rev)
        }),
        ratio_def(StandardMetric::EbitdaMargin, |idx| {
            let oi = idx.resolved(StandardMetric::OperatingIncome)?;
            let da = idx.resolved(StandardMetric::DepreciationAmortization).unwrap_or(0.0);
            let rev = idx.resolved(StandardMetric::Revenue)?;
            div(oi + da, rev)
        }),
        ratio_def(StandardMetric::RevenuePerShare, |idx| {
            div(idx.resolved(StandardMetric::Revenue)?, idx.resolved(StandardMetric::SharesOutstandingBasic)?)
        }),
        ratio_def(StandardMetric::FreeCashFlowPerShare, |idx| {
            let ocf = idx.resolved(StandardMetric::OperatingCashFlow)?;
            let capex = idx.resolved(StandardMetric::CapitalExpenditures).unwrap_or(0.0);
            let shares = idx.resolved(StandardMetric::SharesOutstandingBasic)?;
            div(ocf - capex.abs(), shares)
        }),
        ratio_def(StandardMetric::WorkingCapital, |idx| {
            let ca = idx.resolved(StandardMetric::TotalCurrentAssets)?;
            let cl = idx.resolved(StandardMetric::TotalCurrentLiabilities)?;
            Some(ca - cl)
        }),
        ratio_def(StandardMetric::TangibleBookValue, |idx| {
            let eq = idx.resolved(StandardMetric::TotalStockholdersEquity)?;
            let gw = idx.resolved(StandardMetric::Goodwill).unwrap_or(0.0);
            let ia = idx.resolved(StandardMetric::IntangibleAssets).unwrap_or(0.0);
            Some(eq - gw - ia)
        }),
        ratio_def(StandardMetric::NetDebt, |idx| {
            let ltd = idx.resolved(StandardMetric::LongTermDebt).unwrap_or(0.0);
            let std = idx.resolved(StandardMetric::ShortTermDebt).unwrap_or(0.0);
            let cash = idx.resolved(StandardMetric::CashAndEquivalents)?;
            Some(ltd + std - cash)
        }),
        ratio_def(StandardMetric::BookValuePerShare, |idx| {
            div(idx.resolved(StandardMetric::TotalStockholdersEquity)?, idx.resolved(StandardMetric::SharesOutstandingBasic)?)
        }),
        ratio_def(StandardMetric::PayoutRatio, |idx| {
            div(idx.resolved(StandardMetric::DividendsPaid)?.abs(), idx.resolved(StandardMetric::NetIncome)?)
        }),
        // Computed metrics
        ratio_def(StandardMetric::FreeCashFlow, |idx| {
            let ocf = idx.resolved(StandardMetric::OperatingCashFlow)?;
            let capex = idx.resolved(StandardMetric::CapitalExpenditures).unwrap_or(0.0);
            Some(ocf - capex.abs())
        }),
        ratio_def(StandardMetric::Ebitda, |idx| {
            let oi = idx.resolved(StandardMetric::OperatingIncome)?;
            let da = idx.resolved(StandardMetric::DepreciationAmortization).unwrap_or(0.0);
            Some(oi + da)
        }),
        ratio_def(StandardMetric::Ebit, |idx| {
            idx.resolved(StandardMetric::OperatingIncome)
        }),
    ]
}

fn ratio_def(
    metric: StandardMetric,
    f: impl Fn(&FactIndex) -> Option<f64> + Send + Sync + 'static,
) -> MetricDefinition {
    MetricDefinition {
        metric,
        resolution: MetricResolution::Custom(Arc::new(f)),
    }
}

fn div(a: f64, b: f64) -> Option<f64> {
    if b.abs() < 1e-10 {
        None
    } else {
        Some(a / b)
    }
}
