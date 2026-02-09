use crate::models::company_facts::FactValue;

/// Deduplicate fact values for the same period.
///
/// Strategy:
/// 1. Prefer facts with a `frame` field (SEC's "best fit" dedup).
/// 2. For remaining dupes (same end date): prefer latest `filed` date.
/// 3. Prefer periodic forms (10-K, 10-Q) over other forms (8-K, etc.).
pub fn dedup_facts(facts: &[FactValue]) -> Vec<&FactValue> {
    use std::collections::HashMap;

    // Group by end date
    let mut by_end: HashMap<&str, Vec<&FactValue>> = HashMap::new();
    for fact in facts {
        by_end.entry(&fact.end).or_default().push(fact);
    }

    let mut result = Vec::new();

    for (_end, group) in by_end {
        // If any fact has a frame, prefer those
        let framed: Vec<&FactValue> = group.iter().filter(|f| f.frame.is_some()).copied().collect();
        if !framed.is_empty() {
            // Among framed facts, pick the one filed most recently
            if let Some(best) = pick_best(&framed) {
                result.push(best);
                continue;
            }
        }

        // Otherwise pick the best from all
        if let Some(best) = pick_best(&group) {
            result.push(best);
        }
    }

    // Sort by end date
    result.sort_by(|a, b| a.end.cmp(&b.end));
    result
}

fn pick_best<'a>(facts: &[&'a FactValue]) -> Option<&'a FactValue> {
    if facts.is_empty() {
        return None;
    }

    let mut best = facts[0];
    for &fact in &facts[1..] {
        // Prefer periodic forms
        let best_periodic = is_periodic_form(best);
        let fact_periodic = is_periodic_form(fact);

        if fact_periodic && !best_periodic {
            best = fact;
            continue;
        }
        if best_periodic && !fact_periodic {
            continue;
        }

        // Prefer latest filed date
        if fact.filed > best.filed {
            best = fact;
        }
    }

    Some(best)
}

fn is_periodic_form(fact: &FactValue) -> bool {
    matches!(
        fact.form.as_deref(),
        Some("10-K") | Some("10-K/A") | Some("10-Q") | Some("10-Q/A") | Some("20-F") | Some("40-F")
    )
}
