//! Batch review tool for medium/low confidence discovered tags.
//!
//! Loads `discovered_tags.json`, filters to medium/low LLM entries that haven't
//! been reviewed yet, sends batches to Claude for validation, and applies decisions.
//!
//! Usage:
//!   ANTHROPIC_API_KEY=... cargo run --release --example review_tags
//!
//! Supports resume: progress is saved to `review_progress.json` after each batch.

use std::collections::HashMap;
use std::path::Path;

use edgar_lib::standardizer::learned_tags::{LearnedTagEntry, LearnedTagStore};
use edgar_lib::standardizer::output::StandardMetric;

const TAG_STORE_PATH: &str = "discovered_tags.json";
const PROGRESS_PATH: &str = "review_progress.json";
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_MODEL: &str = "claude-haiku-4-5-20251001";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;
const BATCH_SIZE: usize = 25;

/// A single review decision from Claude (or pre-filter).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ReviewDecision {
    /// Key: "{metric_debug}|{taxonomy}|{tag}"
    key: String,
    decision: String, // "approve", "reclassify", "keep", "garbage"
    #[serde(skip_serializing_if = "Option::is_none")]
    correct_metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    reason: String,
}

/// Claude's per-entry response.
#[derive(Debug, serde::Deserialize)]
struct ClaudeReviewEntry {
    index: usize,
    decision: String,
    #[serde(default)]
    correct_metric: Option<String>,
    #[serde(default)]
    category: Option<String>,
    reason: String,
}

fn entry_key(entry: &LearnedTagEntry) -> String {
    format!("{:?}|{}|{}", entry.metric, entry.taxonomy, entry.tag)
}

/// Returns true if the tag looks like garbage (not a valid XBRL tag).
fn is_garbage(entry: &LearnedTagEntry) -> bool {
    let tag = entry.tag.trim();
    if tag.is_empty() || tag == "NOT_FOUND" || tag == "N/A" || tag == "None" || tag == "null" {
        return true;
    }
    // Tags with spaces are likely natural language, not XBRL identifiers
    if tag.contains(' ') {
        return true;
    }
    // Tags that are all lowercase are suspicious (XBRL uses PascalCase)
    if tag.len() > 3 && tag == tag.to_lowercase() {
        return true;
    }
    false
}

fn build_metric_descriptions() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Income Statement
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
    // Balance Sheet
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
    // Cash Flow
    m.insert("OperatingCashFlow", "Net cash from operating activities");
    m.insert("CapitalExpenditures", "Capital expenditures (purchases of PP&E)");
    m.insert("FreeCashFlow", "Free cash flow (operating - capex)");
    m.insert("InvestingCashFlow", "Net cash from investing activities");
    m.insert("FinancingCashFlow", "Net cash from financing activities");
    m.insert("DividendsPaid", "Dividends paid to shareholders");
    m.insert("ShareRepurchases", "Stock/share repurchases (buybacks)");
    m.insert("NetChangeInCash", "Net increase/decrease in cash");
    // Per Share
    m.insert("EarningsPerShareBasic", "Basic earnings per share");
    m.insert("EarningsPerShareDiluted", "Diluted earnings per share");
    m.insert("BookValuePerShare", "Book value per share");
    m.insert("DividendsPerShare", "Dividends per share");
    m.insert("SharesOutstandingBasic", "Basic shares outstanding / weighted average");
    m.insert("SharesOutstandingDiluted", "Diluted shares outstanding / weighted average");
    // Sector
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
        "You are an SEC EDGAR XBRL taxonomy expert. Your job is to review tag-to-metric mappings \
         that were discovered by an automated scan.\n\n\
         For each entry, you will see:\n\
         - The StandardMetric it was mapped to\n\
         - The XBRL tag name\n\
         - The tag's label (human-readable description)\n\
         - The taxonomy (usually us-gaap)\n\
         - The source entity (company that reported this tag)\n\n\
         Decide whether the tag is a valid alternative for the metric.\n\n\
         VALID alternatives include:\n\
         - Subtypes (e.g. CostOfGoodsSold for CostOfRevenue)\n\
         - Synonyms (e.g. Revenues vs SalesRevenueNet for Revenue)\n\
         - Industry-specific variants (e.g. InterestAndDividendIncomeOperating for Revenue in banking)\n\
         - Broader/narrower scope that still conceptually maps\n\n\
         INVALID mappings include:\n\
         - Opposite direction (income tag mapped to expense metric)\n\
         - Wrong financial statement (cash flow tag for balance sheet metric)\n\
         - Wrong metric entirely (tag clearly belongs to a different metric)\n\
         - Subcomponents that are only a portion of the total\n\n\
         Available StandardMetric values and descriptions:\n"
    );

    let mut sorted: Vec<_> = descriptions.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);
    for (metric, desc) in sorted {
        prompt.push_str(&format!("- {metric}: {desc}\n"));
    }

    prompt.push_str(
        "\nFor each entry, respond with one of:\n\
         - \"approve\": tag is a valid alternative for the assigned metric\n\
         - \"reclassify\": tag doesn't match this metric; provide correct_metric and category\n\
         - \"keep\": genuinely uncertain, needs human review\n\n\
         Reclassification categories:\n\
         - subcomponent: tag is a subset/portion of the metric total\n\
         - opposite_direction: income↔expense or buy↔sell confusion\n\
         - wrong_statement: cash flow tag for balance sheet metric or vice versa\n\
         - wrong_metric: tag clearly maps to a different specific metric\n\
         - unrelated: no meaningful connection to any standard metric\n\n\
         Return ONLY a JSON array (no markdown fencing). Each element:\n\
         {\"index\": 0, \"decision\": \"approve\", \"reason\": \"...\"}\n\
         {\"index\": 1, \"decision\": \"reclassify\", \"correct_metric\": \"InterestExpense\", \
          \"category\": \"opposite_direction\", \"reason\": \"...\"}\n\
         {\"index\": 2, \"decision\": \"keep\", \"reason\": \"...\"}\n"
    );

    prompt
}

fn build_batch_prompt(batch: &[(usize, &LearnedTagEntry)]) -> String {
    let mut prompt = String::from("Review the following tag mappings:\n\n");
    for (i, (_, entry)) in batch.iter().enumerate() {
        let label = entry.label.as_deref().unwrap_or("(no label)");
        prompt.push_str(&format!(
            "{i}. Metric={:?} | Tag={} | Label=\"{label}\" | Taxonomy={} | Entity={}\n",
            entry.metric, entry.tag, entry.taxonomy, entry.source_entity
        ));
    }
    prompt.push_str(&format!(
        "\nReturn a JSON array with exactly {} entries, one per mapping above.\n",
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
        "messages": [
            {
                "role": "user",
                "content": user
            }
        ]
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

    let text = resp_body["content"]
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
        .unwrap_or_default();

    Ok(text)
}

fn parse_claude_response(text: &str) -> Result<Vec<ClaudeReviewEntry>, String> {
    let trimmed = text.trim();
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            return Err("No closing bracket found".to_string());
        }
    } else {
        return Err("No JSON array found in response".to_string());
    };

    serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {e}"))
}

fn load_progress(path: &str) -> Vec<ReviewDecision> {
    if Path::new(path).exists() {
        let data = std::fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_progress(path: &str, decisions: &[ReviewDecision]) {
    let data = serde_json::to_string_pretty(decisions).expect("Failed to serialize progress");
    std::fs::write(path, data).expect("Failed to write progress file");
}

/// Parse a metric variant name string back into a StandardMetric.
fn parse_metric(name: &str) -> Option<StandardMetric> {
    // Use serde deserialization via JSON round-trip
    let json = format!("\"{}\"", name);
    serde_json::from_str::<StandardMetric>(&json).ok()
}

#[tokio::main]
async fn main() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect(
        "ANTHROPIC_API_KEY environment variable required.\n\
         Usage: ANTHROPIC_API_KEY=... cargo run --release --example review_tags"
    );

    // Load tag store
    let store = LearnedTagStore::load(TAG_STORE_PATH).unwrap_or_else(|e| {
        eprintln!("Failed to load {TAG_STORE_PATH}: {e}");
        std::process::exit(1);
    });

    let entries = store.entries();
    println!("Loaded {} total entries from {TAG_STORE_PATH}", entries.len());

    // Filter to unapproved entries that haven't been reviewed yet
    let review_candidates: Vec<(usize, &LearnedTagEntry)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            !e.approved
                && e.category.is_none()
                && e.review_reason.is_none()
        })
        .collect();

    println!("Found {} unreviewed entries to review", review_candidates.len());

    if review_candidates.is_empty() {
        println!("Nothing to review. Exiting.");
        return;
    }

    // Load existing progress
    let mut decisions = load_progress(PROGRESS_PATH);
    let done_keys: std::collections::HashSet<String> =
        decisions.iter().map(|d| d.key.clone()).collect();

    // Pre-filter garbage entries
    let mut garbage_count = 0;
    let mut pending: Vec<(usize, &LearnedTagEntry)> = Vec::new();

    for (idx, entry) in &review_candidates {
        let key = entry_key(entry);
        if done_keys.contains(&key) {
            continue; // Already reviewed
        }

        if is_garbage(entry) {
            decisions.push(ReviewDecision {
                key,
                decision: "garbage".to_string(),
                correct_metric: None,
                category: Some("garbage".to_string()),
                reason: format!("Auto-rejected: invalid tag '{}'", entry.tag),
            });
            garbage_count += 1;
        } else {
            pending.push((*idx, entry));
        }
    }

    if garbage_count > 0 {
        println!("Auto-rejected {garbage_count} garbage entries");
        save_progress(PROGRESS_PATH, &decisions);
    }

    let skipped = review_candidates.len() - garbage_count - pending.len();
    if skipped > 0 {
        println!("Skipping {skipped} already-reviewed entries (resume)");
    }

    println!("{} entries to send to Claude in batches of {BATCH_SIZE}", pending.len());

    if pending.is_empty() {
        println!("All entries already reviewed. Applying decisions...");
    } else {
        // Sort by (metric, confidence) for consistent batching
        pending.sort_by(|a, b| {
            let metric_cmp = format!("{:?}", a.1.metric).cmp(&format!("{:?}", b.1.metric));
            metric_cmp.then_with(|| a.1.confidence.cmp(&b.1.confidence))
        });

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
                Ok(response_text) => {
                    match parse_claude_response(&response_text) {
                        Ok(claude_entries) => {
                            let mut approved = 0;
                            let mut reclassified = 0;
                            let mut kept = 0;

                            for ce in &claude_entries {
                                if ce.index >= chunk.len() {
                                    eprintln!("  Warning: index {} out of range, skipping", ce.index);
                                    continue;
                                }

                                let (_, entry) = chunk[ce.index];
                                let key = entry_key(entry);

                                let category = match ce.decision.as_str() {
                                    "approve" => {
                                        approved += 1;
                                        None
                                    }
                                    "reclassify" => {
                                        reclassified += 1;
                                        Some(ce.category.clone().unwrap_or_else(|| "wrong_metric".to_string()))
                                    }
                                    "keep" => {
                                        kept += 1;
                                        None
                                    }
                                    other => {
                                        eprintln!("  Warning: unknown decision '{other}' for index {}", ce.index);
                                        None
                                    }
                                };

                                decisions.push(ReviewDecision {
                                    key,
                                    decision: ce.decision.clone(),
                                    correct_metric: ce.correct_metric.clone(),
                                    category,
                                    reason: ce.reason.clone(),
                                });
                            }

                            // Handle entries that Claude didn't respond to
                            let responded_indices: std::collections::HashSet<usize> =
                                claude_entries.iter().map(|ce| ce.index).collect();
                            for (i, (_, entry)) in chunk.iter().enumerate() {
                                if !responded_indices.contains(&i) {
                                    let key = entry_key(entry);
                                    if !decisions.iter().any(|d| d.key == key) {
                                        decisions.push(ReviewDecision {
                                            key,
                                            decision: "keep".to_string(),
                                            correct_metric: None,
                                            category: None,
                                            reason: "No response from Claude for this entry".to_string(),
                                        });
                                        kept += 1;
                                    }
                                }
                            }

                            println!(
                                "  Results: {approved} approved, {reclassified} reclassified, {kept} kept"
                            );
                        }
                        Err(e) => {
                            eprintln!("  Failed to parse response: {e}");
                            eprintln!("  Raw response: {}", &response_text[..response_text.len().min(500)]);
                            // Mark all as "keep" so they don't get re-sent
                            for (_, entry) in chunk {
                                let key = entry_key(entry);
                                if !decisions.iter().any(|d| d.key == key) {
                                    decisions.push(ReviewDecision {
                                        key,
                                        decision: "keep".to_string(),
                                        correct_metric: None,
                                        category: None,
                                        reason: format!("Parse error: {e}"),
                                    });
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  API call failed: {e}");
                    // Don't save progress for API failures — will retry on next run
                    eprintln!("  Saving progress and exiting for retry...");
                    save_progress(PROGRESS_PATH, &decisions);
                    std::process::exit(1);
                }
            }

            // Save progress after each batch
            save_progress(PROGRESS_PATH, &decisions);
        }
    }

    // Apply all decisions to the tag store
    println!("\n{}", "=".repeat(60));
    println!("Applying {} decisions to {TAG_STORE_PATH}...", decisions.len());

    let mut store = LearnedTagStore::load(TAG_STORE_PATH).unwrap();
    let entries = store.entries_mut();

    // Build lookup from key -> decision
    let decision_map: HashMap<String, &ReviewDecision> =
        decisions.iter().map(|d| (d.key.clone(), d)).collect();

    let mut applied_approve = 0u32;
    let mut applied_reclassify = 0u32;
    let mut applied_keep = 0u32;
    let mut applied_garbage = 0u32;

    for entry in entries.iter_mut() {
        let key = entry_key(entry);
        if let Some(decision) = decision_map.get(&key) {
            match decision.decision.as_str() {
                "approve" => {
                    entry.approved = true;
                    entry.review_reason = Some(format!("approved: {}", decision.reason));
                    applied_approve += 1;
                }
                "reclassify" => {
                    entry.approved = false;
                    entry.category = decision.category.clone();
                    entry.review_reason = Some(format!("reclassified: {}", decision.reason));
                    // Update metric if Claude identified the correct one
                    if let Some(correct) = &decision.correct_metric {
                        if let Some(metric) = parse_metric(correct) {
                            entry.metric = metric;
                        }
                    }
                    applied_reclassify += 1;
                }
                "keep" => {
                    entry.review_reason = Some(format!("uncertain: {}", decision.reason));
                    applied_keep += 1;
                }
                "garbage" => {
                    entry.approved = false;
                    entry.category = Some("garbage".to_string());
                    entry.review_reason = Some(decision.reason.clone());
                    applied_garbage += 1;
                }
                _ => {}
            }
        }
    }

    store.save().unwrap();

    // Print summary
    println!("\n{}", "=".repeat(60));
    println!("  REVIEW COMPLETE");
    println!("  Total decisions:  {}", decisions.len());
    println!("  Approved:         {applied_approve}");
    println!("  Reclassified:     {applied_reclassify}");
    println!("  Kept (uncertain): {applied_keep}");
    println!("  Garbage:          {applied_garbage}");
    println!();

    // Count final store stats
    let final_entries = LearnedTagStore::load(TAG_STORE_PATH).unwrap();
    let total = final_entries.entries().len();
    let approved = final_entries.entries().iter().filter(|e| e.approved).count();
    let categorized = final_entries
        .entries()
        .iter()
        .filter(|e| e.category.is_some())
        .count();
    let reviewed = final_entries
        .entries()
        .iter()
        .filter(|e| e.review_reason.is_some())
        .count();

    println!("  Store stats:");
    println!("    Total entries:    {total}");
    println!("    Approved:         {approved}");
    println!("    Categorized:      {categorized}");
    println!("    Reviewed:         {reviewed}");
    println!("    Unreviewed:       {}", total - reviewed);
    println!("{}", "=".repeat(60));
}
