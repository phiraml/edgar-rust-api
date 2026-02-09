use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::period::FiscalPeriod;

/// A standardized metric identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StandardMetric {
    // ─── Income Statement ─────────────────────────
    Revenue,
    CostOfRevenue,
    GrossProfit,
    ResearchAndDevelopment,
    SellingGeneralAdmin,
    OperatingExpenses,
    OperatingIncome,
    InterestExpense,
    InterestIncome,
    OtherNonOperatingIncome,
    PretaxIncome,
    IncomeTaxExpense,
    NetIncome,
    NetIncomeToCommon,
    Ebitda,
    Ebit,
    DepreciationAmortization,

    // ─── Balance Sheet ────────────────────────────
    CashAndEquivalents,
    ShortTermInvestments,
    CashAndShortTermInvestments,
    AccountsReceivable,
    Inventory,
    OtherCurrentAssets,
    TotalCurrentAssets,
    PropertyPlantEquipment,
    Goodwill,
    IntangibleAssets,
    OtherNonCurrentAssets,
    TotalAssets,
    AccountsPayable,
    ShortTermDebt,
    CurrentPortionLongTermDebt,
    OtherCurrentLiabilities,
    TotalCurrentLiabilities,
    LongTermDebt,
    OtherNonCurrentLiabilities,
    TotalLiabilities,
    CommonStock,
    RetainedEarnings,
    AccumulatedOtherComprehensiveIncome,
    TotalStockholdersEquity,
    TotalLiabilitiesAndEquity,

    // ─── Cash Flow ────────────────────────────────
    OperatingCashFlow,
    CapitalExpenditures,
    FreeCashFlow,
    InvestingCashFlow,
    FinancingCashFlow,
    DividendsPaid,
    ShareRepurchases,
    NetChangeInCash,

    // ─── Per Share ────────────────────────────────
    EarningsPerShareBasic,
    EarningsPerShareDiluted,
    BookValuePerShare,
    DividendsPerShare,
    SharesOutstandingBasic,
    SharesOutstandingDiluted,

    // ─── Ratios ───────────────────────────────────
    GrossMargin,
    OperatingMargin,
    NetMargin,
    ReturnOnAssets,
    ReturnOnEquity,
    CurrentRatio,
    QuickRatio,
    DebtToEquity,
    DebtToAssets,
    InterestCoverage,
    AssetTurnover,
    InventoryTurnover,
    ReceivablesTurnover,
    FreeCashFlowMargin,
    EbitdaMargin,
    RevenuePerShare,
    FreeCashFlowPerShare,
    PriceToEarnings,
    PriceToBook,
    PriceToSales,
    EvToEbitda,
    PayoutRatio,
    DividendYield,
    WorkingCapital,
    TangibleBookValue,
    NetDebt,

    // ─── Sector-specific ──────────────────────────
    // Banks
    NetInterestIncome,
    NetInterestMargin,
    ProvisionForCreditLosses,
    NonInterestIncome,
    Tier1CapitalRatio,
    TotalCapitalRatio,

    // Insurance
    PremiumsEarned,
    CombinedRatio,
    LossRatio,
    ExpenseRatio,

    // REITs
    FundsFromOperations,
    AdjustedFundsFromOperations,
    NetOperatingIncome,

    /// User-defined custom metric.
    Custom(u32),
}

impl StandardMetric {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Revenue => "Revenue",
            Self::CostOfRevenue => "Cost of Revenue",
            Self::GrossProfit => "Gross Profit",
            Self::ResearchAndDevelopment => "R&D Expense",
            Self::SellingGeneralAdmin => "SG&A Expense",
            Self::OperatingExpenses => "Operating Expenses",
            Self::OperatingIncome => "Operating Income",
            Self::InterestExpense => "Interest Expense",
            Self::InterestIncome => "Interest Income",
            Self::OtherNonOperatingIncome => "Other Non-Operating Income",
            Self::PretaxIncome => "Pre-Tax Income",
            Self::IncomeTaxExpense => "Income Tax Expense",
            Self::NetIncome => "Net Income",
            Self::NetIncomeToCommon => "Net Income to Common",
            Self::Ebitda => "EBITDA",
            Self::Ebit => "EBIT",
            Self::DepreciationAmortization => "Depreciation & Amortization",
            Self::CashAndEquivalents => "Cash & Equivalents",
            Self::ShortTermInvestments => "Short-Term Investments",
            Self::CashAndShortTermInvestments => "Cash & Short-Term Investments",
            Self::AccountsReceivable => "Accounts Receivable",
            Self::Inventory => "Inventory",
            Self::OtherCurrentAssets => "Other Current Assets",
            Self::TotalCurrentAssets => "Total Current Assets",
            Self::PropertyPlantEquipment => "PP&E",
            Self::Goodwill => "Goodwill",
            Self::IntangibleAssets => "Intangible Assets",
            Self::OtherNonCurrentAssets => "Other Non-Current Assets",
            Self::TotalAssets => "Total Assets",
            Self::AccountsPayable => "Accounts Payable",
            Self::ShortTermDebt => "Short-Term Debt",
            Self::CurrentPortionLongTermDebt => "Current Portion of LT Debt",
            Self::OtherCurrentLiabilities => "Other Current Liabilities",
            Self::TotalCurrentLiabilities => "Total Current Liabilities",
            Self::LongTermDebt => "Long-Term Debt",
            Self::OtherNonCurrentLiabilities => "Other Non-Current Liabilities",
            Self::TotalLiabilities => "Total Liabilities",
            Self::CommonStock => "Common Stock",
            Self::RetainedEarnings => "Retained Earnings",
            Self::AccumulatedOtherComprehensiveIncome => "Accumulated OCI",
            Self::TotalStockholdersEquity => "Total Stockholders' Equity",
            Self::TotalLiabilitiesAndEquity => "Total Liabilities & Equity",
            Self::OperatingCashFlow => "Operating Cash Flow",
            Self::CapitalExpenditures => "Capital Expenditures",
            Self::FreeCashFlow => "Free Cash Flow",
            Self::InvestingCashFlow => "Investing Cash Flow",
            Self::FinancingCashFlow => "Financing Cash Flow",
            Self::DividendsPaid => "Dividends Paid",
            Self::ShareRepurchases => "Share Repurchases",
            Self::NetChangeInCash => "Net Change in Cash",
            Self::EarningsPerShareBasic => "EPS (Basic)",
            Self::EarningsPerShareDiluted => "EPS (Diluted)",
            Self::BookValuePerShare => "Book Value Per Share",
            Self::DividendsPerShare => "Dividends Per Share",
            Self::SharesOutstandingBasic => "Shares Outstanding (Basic)",
            Self::SharesOutstandingDiluted => "Shares Outstanding (Diluted)",
            Self::GrossMargin => "Gross Margin",
            Self::OperatingMargin => "Operating Margin",
            Self::NetMargin => "Net Margin",
            Self::ReturnOnAssets => "Return on Assets",
            Self::ReturnOnEquity => "Return on Equity",
            Self::CurrentRatio => "Current Ratio",
            Self::QuickRatio => "Quick Ratio",
            Self::DebtToEquity => "Debt to Equity",
            Self::DebtToAssets => "Debt to Assets",
            Self::InterestCoverage => "Interest Coverage",
            Self::AssetTurnover => "Asset Turnover",
            Self::InventoryTurnover => "Inventory Turnover",
            Self::ReceivablesTurnover => "Receivables Turnover",
            Self::FreeCashFlowMargin => "FCF Margin",
            Self::EbitdaMargin => "EBITDA Margin",
            Self::RevenuePerShare => "Revenue Per Share",
            Self::FreeCashFlowPerShare => "FCF Per Share",
            Self::PriceToEarnings => "P/E Ratio",
            Self::PriceToBook => "P/B Ratio",
            Self::PriceToSales => "P/S Ratio",
            Self::EvToEbitda => "EV/EBITDA",
            Self::PayoutRatio => "Payout Ratio",
            Self::DividendYield => "Dividend Yield",
            Self::WorkingCapital => "Working Capital",
            Self::TangibleBookValue => "Tangible Book Value",
            Self::NetDebt => "Net Debt",
            Self::NetInterestIncome => "Net Interest Income",
            Self::NetInterestMargin => "Net Interest Margin",
            Self::ProvisionForCreditLosses => "Provision for Credit Losses",
            Self::NonInterestIncome => "Non-Interest Income",
            Self::Tier1CapitalRatio => "Tier 1 Capital Ratio",
            Self::TotalCapitalRatio => "Total Capital Ratio",
            Self::PremiumsEarned => "Premiums Earned",
            Self::CombinedRatio => "Combined Ratio",
            Self::LossRatio => "Loss Ratio",
            Self::ExpenseRatio => "Expense Ratio (Insurance)",
            Self::FundsFromOperations => "FFO",
            Self::AdjustedFundsFromOperations => "AFFO",
            Self::NetOperatingIncome => "NOI",
            Self::Custom(_) => "Custom Metric",
        }
    }

    pub fn is_ratio(&self) -> bool {
        matches!(
            self,
            Self::GrossMargin
                | Self::OperatingMargin
                | Self::NetMargin
                | Self::ReturnOnAssets
                | Self::ReturnOnEquity
                | Self::CurrentRatio
                | Self::QuickRatio
                | Self::DebtToEquity
                | Self::DebtToAssets
                | Self::InterestCoverage
                | Self::AssetTurnover
                | Self::InventoryTurnover
                | Self::ReceivablesTurnover
                | Self::FreeCashFlowMargin
                | Self::EbitdaMargin
                | Self::PriceToEarnings
                | Self::PriceToBook
                | Self::PriceToSales
                | Self::EvToEbitda
                | Self::PayoutRatio
                | Self::DividendYield
                | Self::NetInterestMargin
                | Self::CombinedRatio
                | Self::LossRatio
                | Self::ExpenseRatio
                | Self::Tier1CapitalRatio
                | Self::TotalCapitalRatio
        )
    }
}

/// A single resolved metric value for a specific period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub metric: StandardMetric,
    pub value: f64,
    pub unit: String,
    /// Which XBRL tag was used to resolve this value.
    pub source_tag: Option<String>,
}

/// Financial data for a single reporting period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodData {
    pub period: FiscalPeriod,
    pub end_date: Option<String>,
    pub metrics: BTreeMap<StandardMetric, MetricValue>,
}

impl PeriodData {
    pub fn get(&self, metric: &StandardMetric) -> Option<f64> {
        self.metrics.get(metric).map(|m| m.value)
    }
}

/// The complete standardized financial output for a company.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardizedFinancials {
    pub entity_name: String,
    pub cik: u64,
    /// Annual periods, ordered chronologically.
    pub annual: Vec<PeriodData>,
    /// Quarterly periods, ordered chronologically.
    pub quarterly: Vec<PeriodData>,
}

impl StandardizedFinancials {
    /// Get the most recent annual period.
    pub fn latest_annual(&self) -> Option<&PeriodData> {
        self.annual.last()
    }

    /// Get the most recent quarterly period.
    pub fn latest_quarterly(&self) -> Option<&PeriodData> {
        self.quarterly.last()
    }

    /// Get a specific metric from the latest annual period.
    pub fn latest_annual_metric(&self, metric: &StandardMetric) -> Option<f64> {
        self.latest_annual()?.get(metric)
    }
}
