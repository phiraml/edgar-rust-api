//! Third pass: identify the correct metric for rejected tags that don't have a proper home.
//!
//! Takes entries categorized as wrong_metric, wrong_statement, or unrelated and asks
//! Claude: "What StandardMetric does this tag actually belong to, if any?"
//!
//! Usage:
//!   ANTHROPIC_API_KEY=... cargo run --release --example remap_tags

use std::collections::HashMap;
use std::path::Path;

use edgar_lib::standardizer::learned_tags::{LearnedTagEntry, LearnedTagStore};

const TAG_STORE_PATH: &str = "discovered_tags.json";
const PROGRESS_PATH: &str = "remap_progress.json";
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_MODEL: &str = "claude-haiku-4-5-20251001";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;
const BATCH_SIZE: usize = 25;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RemapDecision {
    key: String,
    decision: String, // "map", "subcomponent", "none"
    #[serde(skip_serializing_if = "Option::is_none")]
    correct_metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    reason: String,
}

#[derive(Debug, serde::Deserialize)]
struct ClaudeRemapEntry {
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

fn parse_metric(name: &str) -> Option<edgar_lib::standardizer::output::StandardMetric> {
    let json = format!("\"{}\"", name);
    serde_json::from_str(&json).ok()
}

fn build_system_prompt() -> String {
    String::from(
        "You are an SEC EDGAR XBRL taxonomy expert.\n\n\
         Each entry shows an XBRL tag that was incorrectly mapped to a metric. \
         Your job: identify which StandardMetric this tag ACTUALLY belongs to.\n\n\
         For each tag, decide:\n\
         - \"map\": The tag is a valid primary measure for a specific StandardMetric. \
           Provide correct_metric with the exact variant name.\n\
         - \"subcomponent\": The tag is only a PARTIAL measure of a metric (e.g. SellingExpense \
           is part of SellingGeneralAdmin). Provide correct_metric for which metric it's a subset of, \
           and set category to \"subcomponent\".\n\
         - \"none\": The tag doesn't meaningfully map to any StandardMetric (disclosure items, \
           rates, percentages, fair value tags, schedule items, etc.)\n\n\
         Available StandardMetric values:\n\
         Income Statement: Revenue, CostOfRevenue, GrossProfit, ResearchAndDevelopment, \
         SellingGeneralAdmin, OperatingExpenses, OperatingIncome, InterestExpense, InterestIncome, \
         OtherNonOperatingIncome, PretaxIncome, IncomeTaxExpense, NetIncome, NetIncomeToCommon, \
         Ebitda, Ebit, DepreciationAmortization\n\
         Balance Sheet: CashAndEquivalents, ShortTermInvestments, CashAndShortTermInvestments, \
         AccountsReceivable, Inventory, OtherCurrentAssets, TotalCurrentAssets, PropertyPlantEquipment, \
         Goodwill, IntangibleAssets, OtherNonCurrentAssets, TotalAssets, AccountsPayable, ShortTermDebt, \
         CurrentPortionLongTermDebt, OtherCurrentLiabilities, TotalCurrentLiabilities, LongTermDebt, \
         OtherNonCurrentLiabilities, TotalLiabilities, CommonStock, RetainedEarnings, \
         AccumulatedOtherComprehensiveIncome, TotalStockholdersEquity, TotalLiabilitiesAndEquity\n\
         Cash Flow: OperatingCashFlow, CapitalExpenditures, FreeCashFlow, InvestingCashFlow, \
         FinancingCashFlow, DividendsPaid, ShareRepurchases, NetChangeInCash\n\
         Per Share: EarningsPerShareBasic, EarningsPerShareDiluted, BookValuePerShare, DividendsPerShare, \
         SharesOutstandingBasic, SharesOutstandingDiluted\n\
         Sector: NetInterestIncome, NetInterestMargin, ProvisionForCreditLosses, NonInterestIncome, \
         Tier1CapitalRatio, TotalCapitalRatio, PremiumsEarned, CombinedRatio, LossRatio, ExpenseRatio, \
         FundsFromOperations, AdjustedFundsFromOperations, NetOperatingIncome\n\n\
         Return ONLY a JSON array (no markdown fencing):\n\
         {\"index\": 0, \"decision\": \"map\", \"correct_metric\": \"InterestExpense\", \"reason\": \"...\"}\n\
         {\"index\": 1, \"decision\": \"subcomponent\", \"correct_metric\": \"OperatingExpenses\", \
          \"category\": \"subcomponent\", \"reason\": \"...\"}\n\
         {\"index\": 2, \"decision\": \"none\", \"category\": \"unrelated\", \"reason\": \"disclosure item, not a financial statement line\"}\n"
    )
}

fn build_batch_prompt(batch: &[(usize, &LearnedTagEntry)]) -> String {
    let mut prompt = String::from(
        "For each XBRL tag below, identify which StandardMetric it actually belongs to.\n\
         The 'Currently' field shows what it was incorrectly mapped to.\n\n",
    );
    for (i, (_, entry)) in batch.iter().enumerate() {
        let label = entry.label.as_deref().unwrap_or("(no label)");
        prompt.push_str(&format!(
            "{i}. Tag={} | Label=\"{label}\" | Taxonomy={} | Currently={:?}\n",
            entry.tag, entry.taxonomy, entry.metric
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
        return Err(format!("API error ({status}): {body}"));
    }

    let resp: serde_json::Value = response.json().await.map_err(|e| format!("JSON: {e}"))?;
    Ok(resp["content"]
        .as_array()
        .and_then(|b| b.iter().find_map(|b| {
            if b["type"].as_str() == Some("text") { b["text"].as_str().map(String::from) } else { None }
        }))
        .unwrap_or_default())
}

fn parse_response(text: &str) -> Result<Vec<ClaudeRemapEntry>, String> {
    let t = text.trim();
    let json = if let (Some(s), Some(e)) = (t.find('['), t.rfind(']')) {
        &t[s..=e]
    } else {
        return Err("No JSON array".into());
    };
    serde_json::from_str(json).map_err(|e| format!("Parse: {e}"))
}

fn load_progress(path: &str) -> Vec<RemapDecision> {
    if Path::new(path).exists() {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_default()).unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_progress(path: &str, decisions: &[RemapDecision]) {
    std::fs::write(path, serde_json::to_string_pretty(decisions).unwrap()).unwrap();
}

#[tokio::main]
async fn main() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY required");

    let store = LearnedTagStore::load(TAG_STORE_PATH).unwrap_or_else(|e| {
        eprintln!("Failed to load: {e}");
        std::process::exit(1);
    });

    let candidates: Vec<(usize, &LearnedTagEntry)> = store
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            !e.approved
                && matches!(
                    e.category.as_deref(),
                    Some("wrong_metric") | Some("wrong_statement") | Some("unrelated")
                )
        })
        .collect();

    println!("Found {} entries to remap", candidates.len());

    let mut decisions = load_progress(PROGRESS_PATH);
    let done: std::collections::HashSet<String> = decisions.iter().map(|d| d.key.clone()).collect();

    let pending: Vec<_> = candidates
        .into_iter()
        .filter(|(_, e)| !done.contains(&entry_key(e)))
        .collect();

    if !done.is_empty() {
        println!("Resuming: {} done, {} remaining", done.len(), pending.len());
    }

    if !pending.is_empty() {
        let system = build_system_prompt();
        let client = reqwest::Client::new();
        let total_batches = (pending.len() + BATCH_SIZE - 1) / BATCH_SIZE;

        for (batch_num, chunk) in pending.chunks(BATCH_SIZE).enumerate() {
            println!(
                "\nBatch {}/{total_batches} ({} entries)...",
                batch_num + 1,
                chunk.len()
            );

            match call_claude(&client, &api_key, &system, &build_batch_prompt(chunk)).await {
                Ok(text) => match parse_response(&text) {
                    Ok(entries) => {
                        let mut mapped = 0u32;
                        let mut sub = 0u32;
                        let mut none = 0u32;

                        for ce in &entries {
                            if ce.index >= chunk.len() { continue; }
                            let (_, entry) = chunk[ce.index];
                            match ce.decision.as_str() {
                                "map" => mapped += 1,
                                "subcomponent" => sub += 1,
                                _ => none += 1,
                            }
                            decisions.push(RemapDecision {
                                key: entry_key(entry),
                                decision: ce.decision.clone(),
                                correct_metric: ce.correct_metric.clone(),
                                category: ce.category.clone(),
                                reason: ce.reason.clone(),
                            });
                        }

                        let responded: std::collections::HashSet<usize> =
                            entries.iter().map(|e| e.index).collect();
                        for (i, (_, entry)) in chunk.iter().enumerate() {
                            if !responded.contains(&i) && !decisions.iter().any(|d| d.key == entry_key(entry)) {
                                decisions.push(RemapDecision {
                                    key: entry_key(entry),
                                    decision: "none".to_string(),
                                    correct_metric: None,
                                    category: Some("unrelated".to_string()),
                                    reason: "No response".to_string(),
                                });
                                none += 1;
                            }
                        }
                        println!("  {mapped} mapped, {sub} subcomponent, {none} none");
                    }
                    Err(e) => {
                        eprintln!("  Parse error: {e}");
                        for (_, entry) in chunk {
                            let key = entry_key(entry);
                            if !decisions.iter().any(|d| d.key == key) {
                                decisions.push(RemapDecision {
                                    key, decision: "none".to_string(),
                                    correct_metric: None,
                                    category: Some("unrelated".to_string()),
                                    reason: format!("Parse error: {e}"),
                                });
                            }
                        }
                    }
                },
                Err(e) => {
                    eprintln!("  API error: {e}");
                    save_progress(PROGRESS_PATH, &decisions);
                    std::process::exit(1);
                }
            }
            save_progress(PROGRESS_PATH, &decisions);
        }
    }

    // Apply
    println!("\n{}", "=".repeat(60));
    println!("Applying {} remap decisions...", decisions.len());

    let mut store = LearnedTagStore::load(TAG_STORE_PATH).unwrap();
    let dmap: HashMap<String, &RemapDecision> = decisions.iter().map(|d| (d.key.clone(), d)).collect();

    let mut applied_map = 0u32;
    let mut applied_sub = 0u32;
    let mut applied_none = 0u32;

    for entry in store.entries_mut().iter_mut() {
        let key = entry_key(entry);
        if let Some(dec) = dmap.get(&key) {
            match dec.decision.as_str() {
                "map" => {
                    if let Some(m) = dec.correct_metric.as_ref().and_then(|n| parse_metric(n)) {
                        entry.metric = m;
                        entry.approved = true;
                        entry.category = None;
                        entry.review_reason = Some(format!("remapped: {}", dec.reason));
                        applied_map += 1;
                    } else {
                        entry.category = Some("unrelated".to_string());
                        entry.review_reason = Some(format!("remap-failed: invalid metric name '{}'", dec.correct_metric.as_deref().unwrap_or("?")));
                        applied_none += 1;
                    }
                }
                "subcomponent" => {
                    if let Some(m) = dec.correct_metric.as_ref().and_then(|n| parse_metric(n)) {
                        entry.metric = m;
                    }
                    entry.category = Some("subcomponent".to_string());
                    entry.review_reason = Some(format!("subcomponent: {}", dec.reason));
                    applied_sub += 1;
                }
                _ => {
                    entry.category = Some(dec.category.clone().unwrap_or("unrelated".to_string()));
                    entry.review_reason = Some(format!("unmappable: {}", dec.reason));
                    applied_none += 1;
                }
            }
        }
    }

    store.save().unwrap();

    let final_store = LearnedTagStore::load(TAG_STORE_PATH).unwrap();
    let total = final_store.entries().len();
    let approved = final_store.entries().iter().filter(|e| e.approved).count();
    let categorized = final_store.entries().iter().filter(|e| e.category.is_some()).count();

    println!("\n{}", "=".repeat(60));
    println!("  REMAP COMPLETE");
    println!("  Mapped (approved):    {applied_map}");
    println!("  Subcomponent:         {applied_sub}");
    println!("  Unmappable:           {applied_none}");
    println!();
    println!("  Store: {total} total, {approved} approved, {categorized} categorized");
    println!("{}", "=".repeat(60));
}
