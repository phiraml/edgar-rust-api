use tabled::settings::Style;
use tabled::{Table, Tabled};

use crate::error::Result;
use crate::models::company::Company;
use crate::models::filing::Filing;
use crate::standardizer::output::{StandardMetric, StandardizedFinancials};

/// Output format selection.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

pub fn print_company(company: &Company, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(company)?);
        }
        _ => {
            println!("Company: {}", company.name);
            println!("CIK:     {}", company.cik);
            if !company.tickers.is_empty() {
                println!("Tickers: {}", company.tickers.join(", "));
            }
            if !company.exchanges.is_empty() {
                println!("Exchanges: {}", company.exchanges.join(", "));
            }
            if let Some(ref sic) = company.sic {
                print!("SIC:     {sic}");
                if let Some(ref desc) = company.sic_description {
                    print!(" — {desc}");
                }
                println!();
            }
            if let Some(ref fye) = company.fiscal_year_end {
                println!("Fiscal Year End: {fye}");
            }
            if let Some(ref state) = company.state_of_incorporation {
                println!("State:   {state}");
            }
            if let Some(ref website) = company.website {
                println!("Website: {website}");
            }
        }
    }
    Ok(())
}

#[derive(Tabled)]
struct FilingRow {
    #[tabled(rename = "Date")]
    date: String,
    #[tabled(rename = "Type")]
    form: String,
    #[tabled(rename = "Accession")]
    accession: String,
    #[tabled(rename = "Description")]
    description: String,
}

pub fn print_filings(filings: &[Filing], format: OutputFormat, limit: usize) -> Result<()> {
    let filings = if limit > 0 && filings.len() > limit {
        &filings[..limit]
    } else {
        filings
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(filings)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(std::io::stdout());
            for f in filings {
                wtr.write_record(&[
                    f.filing_date.to_string(),
                    f.filing_type.to_string(),
                    f.accession_number.clone(),
                    f.primary_doc_description.clone().unwrap_or_default(),
                ])?;
            }
            wtr.flush()?;
        }
        OutputFormat::Table => {
            let rows: Vec<FilingRow> = filings
                .iter()
                .map(|f| FilingRow {
                    date: f.filing_date.to_string(),
                    form: f.filing_type.to_string(),
                    accession: f.accession_number.clone(),
                    description: f.primary_doc_description
                        .clone()
                        .unwrap_or_default(),
                })
                .collect();

            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{table}");
        }
    }
    Ok(())
}

pub fn print_financials(
    financials: &StandardizedFinancials,
    format: OutputFormat,
    show_ratios: bool,
    quarterly: bool,
) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(financials)?);
        }
        _ => {
            println!("Financials for: {}", financials.entity_name);
            println!();

            let periods = if quarterly {
                &financials.quarterly
            } else {
                &financials.annual
            };

            if periods.is_empty() {
                println!("No data available.");
                return Ok(());
            }

            // Show the last 5 periods max
            let start = if periods.len() > 5 {
                periods.len() - 5
            } else {
                0
            };
            let periods = &periods[start..];

            // Collect all metrics across periods
            let mut all_metrics: Vec<StandardMetric> = Vec::new();
            for period in periods {
                for metric in period.metrics.keys() {
                    if !all_metrics.contains(metric) {
                        if show_ratios || !metric.is_ratio() {
                            all_metrics.push(metric.clone());
                        }
                    }
                }
            }

            // Build header
            let mut header = vec!["Metric".to_string()];
            for period in periods {
                header.push(period.period.to_string());
            }

            // Print header
            print!("{:<35}", "Metric");
            for period in periods {
                print!("{:>18}", period.period.to_string());
            }
            println!();
            println!("{}", "-".repeat(35 + 18 * periods.len()));

            // Print each metric
            for metric in &all_metrics {
                print!("{:<35}", metric.display_name());
                for period in periods {
                    if let Some(mv) = period.metrics.get(metric) {
                        let formatted = format_value(mv.value, &mv.unit);
                        print!("{:>18}", formatted);
                    } else {
                        print!("{:>18}", "—");
                    }
                }
                println!();
            }
        }
    }
    Ok(())
}

fn format_value(value: f64, unit: &str) -> String {
    match unit {
        "ratio" => format!("{:.2}%", value * 100.0),
        "USD/shares" => format!("${:.2}", value),
        "shares" => {
            if value.abs() >= 1_000_000_000.0 {
                format!("{:.2}B", value / 1_000_000_000.0)
            } else if value.abs() >= 1_000_000.0 {
                format!("{:.2}M", value / 1_000_000.0)
            } else {
                format!("{:.0}", value)
            }
        }
        _ => {
            // USD amounts
            if value.abs() >= 1_000_000_000.0 {
                format!("${:.2}B", value / 1_000_000_000.0)
            } else if value.abs() >= 1_000_000.0 {
                format!("${:.2}M", value / 1_000_000.0)
            } else if value.abs() >= 1_000.0 {
                format!("${:.0}K", value / 1_000.0)
            } else {
                format!("${:.2}", value)
            }
        }
    }
}
