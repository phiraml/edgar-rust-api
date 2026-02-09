//! Second-pass review: verify reclassified tags against their corrected metrics.
//!
//! Entries that were reclassified (wrong_metric, opposite_direction, wrong_statement)
//! had their metric field updated to what Claude thought was correct. This script
//! re-checks: "Is this tag actually valid for the corrected metric?"
//!
//! Usage:
//!   ANTHROPIC_API_KEY=... cargo run --release --example recheck_tags

use std::collections::HashMap;
use std::path::Path;

use edgar_lib::standardizer::learned_tags::{LearnedTagEntry, LearnedTagStore};

const TAG_STORE_PATH: &str = "discovered_tags.json";
const PROGRESS_PATH: &str = "recheck_progress.json";
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_MODEL: &str = "claude-haiku-4-5-20251001";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;
const BATCH_SIZE: usize = 25;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RecheckDecision {
    key: String,
    decision: String, // "approve" or "reject"
    reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    better_category: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ClaudeRecheckEntry {
    index: usize,
    decision: String, // "approve" or "reject"
    reason: String,
    #[serde(default)]
    better_category: Option<String>,
}

fn entry_key(entry: &LearnedTagEntry) -> String {
    format!("{:?}|{}|{}", entry.metric, entry.taxonomy, entry.tag)
}

fn build_metric_descriptions() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("Revenue", "Total revenue/sales from operations");
    m.insert("CostOfRevenue", "Cost of goods/services sold (COGS)");
    m.insert("GrossProfit", "Revenue minus cost of revenue");
    m.insert("ResearchAndDevelopment", "R&D expenses, technology costs");
    m.insert("SellingGeneralAdmin", "SG&A: selling, general & administrative expenses");
    m.insert("OperatingExpenses", "Total operating expenses");
    m.insert("OperatingIncome", "Operating income/loss");
    m.insert("InterestExpense", "Interest expense on debt");
    m.insert("InterestIncome", "Interest/investment income");
    m.insert("OtherNonOperatingIncome", "Other non-operating income/expense");
    m.insert("PretaxIncome", "Income before income taxes");
    m.insert("IncomeTaxExpense", "Income tax expense/benefit");
    m.insert("NetIncome", "Net income/loss");
    m.insert("NetIncomeToCommon", "Net income available to common shareholders");
    m.insert("Ebitda", "Earnings before interest, taxes, depreciation, amortization");
    m.insert("Ebit", "Earnings before interest and taxes");
    m.insert("DepreciationAmortization", "Depreciation & amortization expense");
    m.insert("CashAndEquivalents", "Cash and cash equivalents");
    m.insert("ShortTermInvestments", "Short-term/marketable securities");
    m.insert("CashAndShortTermInvestments", "Cash + short-term investments combined");
    m.insert("AccountsReceivable", "Accounts receivable, net");
    m.insert("Inventory", "Inventory");
    m.insert("OtherCurrentAssets", "Other current assets");
    m.insert("TotalCurrentAssets", "Total current assets");
    m.insert("PropertyPlantEquipment", "Property, plant & equipment (PP&E), net");
    m.insert("Goodwill", "Goodwill from acquisitions");
    m.insert("IntangibleAssets", "Intangible assets, net");
    m.insert("OtherNonCurrentAssets", "Other non-current/long-term assets");
    m.insert("TotalAssets", "Total assets");
    m.insert("AccountsPayable", "Accounts payable");
    m.insert("ShortTermDebt", "Short-term borrowings/debt");
    m.insert("CurrentPortionLongTermDebt", "Current portion of long-term debt");
    m.insert("OtherCurrentLiabilities", "Other current liabilities");
    m.insert("TotalCurrentLiabilities", "Total current liabilities");
    m.insert("LongTermDebt", "Long-term debt");
    m.insert("OtherNonCurrentLiabilities", "Other non-current liabilities");
    m.insert("TotalLiabilities", "Total liabilities");
    m.insert("CommonStock", "Common stock / additional paid-in capital");
    m.insert("RetainedEarnings", "Retained earnings / accumulated deficit");
    m.insert("AccumulatedOtherComprehensiveIncome", "Accumulated other comprehensive income (AOCI)");
    m.insert("TotalStockholdersEquity", "Total stockholders' equity");
    m.insert("TotalLiabilitiesAndEquity", "Total liabilities and stockholders' equity");
    m.insert("OperatingCashFlow", "Net cash from operating activities");
    m.insert("CapitalExpenditures", "Capital expenditures (purchases of PP&E)");
    m.insert("FreeCashFlow", "Free cash flow (operating - capex)");
    m.insert("InvestingCashFlow", "Net cash from investing activities");
    m.insert("FinancingCashFlow", "Net cash from financing activities");
    m.insert("DividendsPaid", "Dividends paid to shareholders");
    m.insert("ShareRepurchases", "Stock/share repurchases (buybacks)");
    m.insert("NetChangeInCash", "Net increase/decrease in cash");
    m.insert("EarningsPerShareBasic", "Basic earnings per share");
    m.insert("EarningsPerShareDiluted", "Diluted earnings per share");
    m.insert("BookValuePerShare", "Book value per share");
    m.insert("DividendsPerShare", "Dividends per share");
    m.insert("SharesOutstandingBasic", "Basic shares outstanding / weighted average");
    m.insert("SharesOutstandingDiluted", "Diluted shares outstanding / weighted average");
    m.insert("NetInterestIncome", "Net interest income (banks)");
    m.insert("NetInterestMargin", "Net interest margin (banks)");
    m.insert("ProvisionForCreditLosses", "Provision for credit/loan losses (banks)");
    m.insert("NonInterestIncome", "Non-interest income (banks)");
    m.insert("Tier1CapitalRatio", "Tier 1 capital ratio (banks)");
    m.insert("TotalCapitalRatio", "Total capital ratio (banks)");
    m.insert("PremiumsEarned", "Premiums earned (insurance)");
    m.insert("CombinedRatio", "Combined ratio (insurance)");
    m.insert("LossRatio", "Loss ratio (insurance)");
    m.insert("ExpenseRatio", "Expense ratio (insurance)");
    m.insert("FundsFromOperations", "Funds from operations / FFO (REITs)");
    m.insert("AdjustedFundsFromOperations", "Adjusted FFO / AFFO (REITs)");
    m.insert("NetOperatingIncome", "Net operating income / NOI (REITs)");
    m
}

fn build_system_prompt(descriptions: &HashMap<&str, &str>) -> String {
    let mut prompt = String::from(
        "You are an SEC EDGAR XBRL taxonomy expert performing a VALIDATION pass.\n\n\
         Each entry shows an XBRL tag mapped to a StandardMetric. These mappings were \
         previously reclassified — the metric was corrected from a wrong original mapping.\n\n\
         Your job: decide if the tag is a VALID alternative for the metric shown.\n\n\
         APPROVE if:\n\
         - The tag directly measures or represents the metric\n\
         - The tag is a synonym, industry variant, or broader/narrower version that still \
           conceptually maps to the same financial line item\n\
         - The tag would produce a reasonable value for this metric in financial analysis\n\n\
         REJECT if:\n\
         - The tag is a rate, percentage, or ratio but the metric expects a dollar amount\n\
         - The tag is a disclosure/supplementary item, not a primary financial statement line\n\
         - The tag measures something fundamentally different even if related\n\
         - The tag is a subcomponent (only a portion of the metric's total)\n\
         - The tag is from the wrong financial statement (cash flow vs balance sheet)\n\n\
         Available StandardMetric values:\n"
    );

    let mut sorted: Vec<_> = descriptions.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);
    for (metric, desc) in sorted {
        prompt.push_str(&format!("- {metric}: {desc}\n"));
    }

    prompt.push_str(
        "\nReturn ONLY a JSON array (no markdown fencing). Each element:\n\
         {\"index\": 0, \"decision\": \"approve\", \"reason\": \"...\"}\n\
         {\"index\": 1, \"decision\": \"reject\", \"reason\": \"...\", \
          \"better_category\": \"subcomponent|wrong_metric|wrong_statement|unrelated\"}\n"
    );

    prompt
}

fn build_batch_prompt(batch: &[(usize, &LearnedTagEntry)]) -> String {
    let mut prompt = String::from(
        "Validate these tag-to-metric mappings. Is each tag a valid alternative for the metric?\n\n",
    );
    for (i, (_, entry)) in batch.iter().enumerate() {
        let label = entry.label.as_deref().unwrap_or("(no label)");
        prompt.push_str(&format!(
            "{i}. Metric={:?} | Tag={} | Label=\"{label}\" | Taxonomy={}\n",
            entry.metric, entry.tag, entry.taxonomy
        ));
    }
    prompt.push_str(&format!(
        "\nReturn a JSON array with exactly {} entries.\n",
        batch.len()
    ));
    prompt
}

async fn call_claude(
    client: &reqwest::Client,
    api_key: &str,
    system: &str,
    user: &str,
) -> Result<String, String> {
    let request_body = serde_json::json!({
        "model": ANTHROPIC_MODEL,
        "max_tokens": MAX_TOKENS,
        "system": system,
        "messages": [{ "role": "user", "content": user }]
    });

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error ({status}): {body}"));
    }

    let resp_body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {e}"))?;

    Ok(resp_body["content"]
        .as_array()
        .and_then(|blocks| {
            blocks.iter().find_map(|b| {
                if b["type"].as_str() == Some("text") {
                    b["text"].as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default())
}

fn parse_response(text: &str) -> Result<Vec<ClaudeRecheckEntry>, String> {
    let trimmed = text.trim();
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            return Err("No closing bracket".to_string());
        }
    } else {
        return Err("No JSON array found".to_string());
    };
    serde_json::from_str(json_str).map_err(|e| format!("JSON parse: {e}"))
}

fn load_progress(path: &str) -> Vec<RecheckDecision> {
    if Path::new(path).exists() {
        let data = std::fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_progress(path: &str, decisions: &[RecheckDecision]) {
    let data = serde_json::to_string_pretty(decisions).expect("serialize");
    std::fs::write(path, data).expect("write progress");
}

#[tokio::main]
async fn main() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect(
        "ANTHROPIC_API_KEY required.\nUsage: ANTHROPIC_API_KEY=... cargo run --release --example recheck_tags",
    );

    let store = LearnedTagStore::load(TAG_STORE_PATH).unwrap_or_else(|e| {
        eprintln!("Failed to load {TAG_STORE_PATH}: {e}");
        std::process::exit(1);
    });

    // Find reclassified entries (wrong_metric, opposite_direction, wrong_statement)
    let candidates: Vec<(usize, &LearnedTagEntry)> = store
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            !e.approved
                && matches!(
                    e.category.as_deref(),
                    Some("wrong_metric") | Some("opposite_direction") | Some("wrong_statement")
                )
        })
        .collect();

    println!("Found {} reclassified entries to re-check", candidates.len());

    if candidates.is_empty() {
        println!("Nothing to recheck.");
        return;
    }

    let mut decisions = load_progress(PROGRESS_PATH);
    let done_keys: std::collections::HashSet<String> =
        decisions.iter().map(|d| d.key.clone()).collect();

    let pending: Vec<(usize, &LearnedTagEntry)> = candidates
        .into_iter()
        .filter(|(_, e)| !done_keys.contains(&entry_key(e)))
        .collect();

    let skipped = decisions.len();
    if skipped > 0 {
        println!("Resuming: {skipped} already done");
    }
    println!("{} entries to send in batches of {BATCH_SIZE}", pending.len());

    if !pending.is_empty() {
        let descriptions = build_metric_descriptions();
        let system_prompt = build_system_prompt(&descriptions);
        let client = reqwest::Client::new();
        let total_batches = (pending.len() + BATCH_SIZE - 1) / BATCH_SIZE;

        for (batch_num, chunk) in pending.chunks(BATCH_SIZE).enumerate() {
            println!(
                "\nBatch {}/{total_batches} ({} entries)...",
                batch_num + 1,
                chunk.len()
            );

            let user_prompt = build_batch_prompt(chunk);

            match call_claude(&client, &api_key, &system_prompt, &user_prompt).await {
                Ok(text) => match parse_response(&text) {
                    Ok(claude_entries) => {
                        let mut approved = 0;
                        let mut rejected = 0;

                        for ce in &claude_entries {
                            if ce.index >= chunk.len() {
                                continue;
                            }
                            let (_, entry) = chunk[ce.index];
                            match ce.decision.as_str() {
                                "approve" => approved += 1,
                                _ => rejected += 1,
                            }
                            decisions.push(RecheckDecision {
                                key: entry_key(entry),
                                decision: ce.decision.clone(),
                                reason: ce.reason.clone(),
                                better_category: ce.better_category.clone(),
                            });
                        }

                        // Handle missing responses
                        let responded: std::collections::HashSet<usize> =
                            claude_entries.iter().map(|ce| ce.index).collect();
                        for (i, (_, entry)) in chunk.iter().enumerate() {
                            if !responded.contains(&i) {
                                let key = entry_key(entry);
                                if !decisions.iter().any(|d| d.key == key) {
                                    decisions.push(RecheckDecision {
                                        key,
                                        decision: "reject".to_string(),
                                        reason: "No response from Claude".to_string(),
                                        better_category: None,
                                    });
                                    rejected += 1;
                                }
                            }
                        }
                        println!("  {approved} approved, {rejected} rejected");
                    }
                    Err(e) => {
                        eprintln!("  Parse error: {e}");
                        for (_, entry) in chunk {
                            let key = entry_key(entry);
                            if !decisions.iter().any(|d| d.key == key) {
                                decisions.push(RecheckDecision {
                                    key,
                                    decision: "reject".to_string(),
                                    reason: format!("Parse error: {e}"),
                                    better_category: None,
                                });
                            }
                        }
                    }
                },
                Err(e) => {
                    eprintln!("  API error: {e}");
                    eprintln!("  Saving progress for retry...");
                    save_progress(PROGRESS_PATH, &decisions);
                    std::process::exit(1);
                }
            }

            save_progress(PROGRESS_PATH, &decisions);
        }
    }

    // Apply decisions
    println!("\n{}", "=".repeat(60));
    println!("Applying {} recheck decisions...", decisions.len());

    let mut store = LearnedTagStore::load(TAG_STORE_PATH).unwrap();
    let decision_map: HashMap<String, &RecheckDecision> =
        decisions.iter().map(|d| (d.key.clone(), d)).collect();

    let mut applied_approve = 0u32;
    let mut applied_reject = 0u32;

    for entry in store.entries_mut().iter_mut() {
        let key = entry_key(entry);
        if let Some(dec) = decision_map.get(&key) {
            match dec.decision.as_str() {
                "approve" => {
                    entry.approved = true;
                    entry.category = None; // Clear the rejection category
                    entry.review_reason =
                        Some(format!("recheck-approved: {}", dec.reason));
                    applied_approve += 1;
                }
                _ => {
                    // Update category if Claude suggested a better one
                    if let Some(cat) = &dec.better_category {
                        entry.category = Some(cat.clone());
                    }
                    entry.review_reason =
                        Some(format!("recheck-rejected: {}", dec.reason));
                    applied_reject += 1;
                }
            }
        }
    }

    store.save().unwrap();

    // Summary
    let final_store = LearnedTagStore::load(TAG_STORE_PATH).unwrap();
    let total = final_store.entries().len();
    let approved = final_store.entries().iter().filter(|e| e.approved).count();
    let categorized = final_store.entries().iter().filter(|e| e.category.is_some()).count();

    println!("\n{}", "=".repeat(60));
    println!("  RECHECK COMPLETE");
    println!("  Approved:  {applied_approve}");
    println!("  Rejected:  {applied_reject}");
    println!();
    println!("  Store: {total} total, {approved} approved, {categorized} categorized");
    println!("{}", "=".repeat(60));
}
