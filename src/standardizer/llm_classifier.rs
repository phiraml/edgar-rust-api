use serde::{Deserialize, Serialize};

use crate::error::{EdgarError, Result};
use crate::standardizer::coverage::{CandidateTag, MissingMetric};
use crate::standardizer::output::StandardMetric;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_MODEL: &str = "claude-haiku-4-5-20251001";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;

/// A classification result from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmClassification {
    pub metric: String,
    pub tag: String,
    pub taxonomy: String,
    pub confidence: String,
    pub reasoning: String,
}

/// Classifies unmatched XBRL tags against missing standard metrics using Claude.
pub struct LlmClassifier {
    api_key: String,
    client: reqwest::Client,
}

impl LlmClassifier {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Classify unmatched XBRL tags against missing metrics using a single Claude API call.
    ///
    /// `unmatched_tags` is a list of (taxonomy, tag, label, latest_value) tuples for tags
    /// not already mapped to any standard metric.
    pub async fn classify(
        &self,
        entity_name: &str,
        missing: &[&MissingMetric],
        unmatched_tags: &[(String, String, Option<String>, Option<f64>)],
    ) -> Result<Vec<LlmClassification>> {
        if missing.is_empty() || unmatched_tags.is_empty() {
            return Ok(vec![]);
        }

        let prompt = build_prompt(entity_name, missing, unmatched_tags);

        let request_body = serde_json::json!({
            "model": ANTHROPIC_MODEL,
            "max_tokens": MAX_TOKENS,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(EdgarError::Http)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(EdgarError::Other(format!(
                "Anthropic API error ({status}): {body}"
            )));
        }

        let resp_body: serde_json::Value = response.json().await.map_err(EdgarError::Http)?;

        // Extract text from the response
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

        parse_classifications(&text)
    }
}

fn build_prompt(
    entity_name: &str,
    missing: &[&MissingMetric],
    unmatched_tags: &[(String, String, Option<String>, Option<f64>)],
) -> String {
    let mut prompt = format!(
        "You are analyzing XBRL financial data for {entity_name}.\n\
         The following standard financial metrics could not be resolved from known XBRL tags:\n\n"
    );

    for m in missing {
        prompt.push_str(&format!("- {} ({})\n", m.display_name, metric_variant_name(&m.metric)));
    }

    prompt.push_str(
        "\nThe company reports these XBRL tags that are not mapped to any standard metric:\n\n",
    );

    // Limit to top 100 unmatched tags to keep prompt size reasonable
    let limit = unmatched_tags.len().min(100);
    for (taxonomy, tag, label, value) in &unmatched_tags[..limit] {
        let label_str = label.as_deref().unwrap_or("(no label)");
        let val_str = match value {
            Some(v) => format!("${:.0}", v),
            None => "N/A".to_string(),
        };
        prompt.push_str(&format!(
            "- {taxonomy}:{tag} | Label: {label_str} | Latest annual value: {val_str}\n"
        ));
    }
    if unmatched_tags.len() > limit {
        prompt.push_str(&format!("  ... and {} more tags\n", unmatched_tags.len() - limit));
    }

    prompt.push_str(
        "\nFor each missing metric, identify which unmatched tag(s) likely contain that data.\n\
         Return ONLY a JSON array (no markdown fencing) of classifications:\n\
         [{\"metric\": \"MetricVariantName\", \"tag\": \"XBRLTagName\", \"taxonomy\": \"us-gaap\", \
         \"confidence\": \"high|medium|low\", \"reasoning\": \"brief explanation\"}]\n\
         Only include matches you are confident about. It's fine to return an empty array [].\n"
    );

    prompt
}

/// Convert a StandardMetric to its variant name string for the prompt.
fn metric_variant_name(metric: &StandardMetric) -> String {
    format!("{metric:?}")
}

/// Parse the LLM response text into classification structs.
fn parse_classifications(text: &str) -> Result<Vec<LlmClassification>> {
    // Find JSON array in the response (handle potential markdown fencing)
    let trimmed = text.trim();
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            return Ok(vec![]);
        }
    } else {
        return Ok(vec![]);
    };

    serde_json::from_str(json_str).map_err(|e| {
        EdgarError::Other(format!("Failed to parse LLM classifications: {e}"))
    })
}

/// Strip taxonomy prefix from a tag name if the LLM included it.
/// e.g. "us-gaap:RetainedEarningsAccumulatedDeficit" → "RetainedEarningsAccumulatedDeficit"
fn strip_taxonomy_prefix(tag: &str) -> &str {
    tag.rsplit_once(':').map(|(_, t)| t).unwrap_or(tag)
}

/// Convert an LLM classification into a CandidateTag to merge into a MissingMetric.
pub fn classification_to_candidate(
    classification: &LlmClassification,
    unmatched_tags: &[(String, String, Option<String>, Option<f64>)],
) -> Option<CandidateTag> {
    let clean_tag = strip_taxonomy_prefix(&classification.tag).to_string();
    let clean_taxonomy = strip_taxonomy_prefix(&classification.taxonomy).to_string();

    // Find the matching unmatched tag to get its label and value
    let found = unmatched_tags.iter().find(|(tax, tag, _, _)| {
        tax == &clean_taxonomy && tag == &clean_tag
    });

    let (label, latest_value, unit) = match found {
        Some((_, _, label, value)) => (label.clone(), *value, "USD".to_string()),
        None => (None, None, "USD".to_string()),
    };

    Some(CandidateTag {
        taxonomy: clean_taxonomy,
        tag: clean_tag,
        label,
        latest_value,
        unit,
        match_reason: format!(
            "llm: {} (confidence: {})",
            classification.reasoning, classification.confidence
        ),
    })
}
