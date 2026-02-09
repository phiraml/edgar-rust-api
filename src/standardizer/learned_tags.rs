use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{EdgarError, Result};
use crate::standardizer::catalog::MetricDefinition;
use crate::standardizer::output::StandardMetric;
use crate::standardizer::resolution::{MetricResolution, TagSpec};

/// A single learned tag entry persisted to the JSON store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedTagEntry {
    pub metric: StandardMetric,
    pub taxonomy: String,
    pub tag: String,
    pub unit: String,
    pub label: Option<String>,
    pub confidence: String,
    pub source: String,
    pub source_entity: String,
    pub discovered_at: String,
    pub approved: bool,
    /// Classification category for rejected/reclassified entries (e.g. "opposite_direction", "garbage").
    /// `None` means the entry has not been categorized as wrong.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Explanation from the review process (approval reason, rejection reason, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_reason: Option<String>,
}

/// Persistent store for learned XBRL tag mappings.
#[derive(Debug)]
pub struct LearnedTagStore {
    path: PathBuf,
    entries: Vec<LearnedTagEntry>,
}

impl LearnedTagStore {
    /// Load the store from a JSON file. Returns an empty store if the file doesn't exist.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if path.exists() {
            let data = std::fs::read_to_string(&path).map_err(|e| {
                EdgarError::Other(format!("Failed to read learned tags from {}: {e}", path.display()))
            })?;
            let entries: Vec<LearnedTagEntry> = serde_json::from_str(&data).map_err(|e| {
                EdgarError::Other(format!(
                    "Failed to parse learned tags from {}: {e}",
                    path.display()
                ))
            })?;
            Ok(Self { path, entries })
        } else {
            Ok(Self {
                path,
                entries: Vec::new(),
            })
        }
    }

    /// Save the store back to disk.
    pub fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.entries).map_err(|e| {
            EdgarError::Other(format!("Failed to serialize learned tags: {e}"))
        })?;
        std::fs::write(&self.path, data).map_err(|e| {
            EdgarError::Other(format!(
                "Failed to write learned tags to {}: {e}",
                self.path.display()
            ))
        })?;
        Ok(())
    }

    /// Add an entry, deduplicating by (metric, taxonomy, tag).
    /// Strips any taxonomy prefix the LLM may have included in the tag/taxonomy fields
    /// (e.g. "us-gaap:SomeTag" → "SomeTag").
    pub fn add(&mut self, mut entry: LearnedTagEntry) {
        // Strip doubled taxonomy prefixes from LLM responses
        if let Some((_prefix, clean)) = entry.tag.rsplit_once(':') {
            entry.tag = clean.to_string();
        }
        if let Some((_prefix, clean)) = entry.taxonomy.rsplit_once(':') {
            entry.taxonomy = clean.to_string();
        }

        let exists = self.entries.iter().any(|e| {
            e.metric == entry.metric && e.taxonomy == entry.taxonomy && e.tag == entry.tag
        });
        if !exists {
            self.entries.push(entry);
        }
    }

    /// Get all entries.
    pub fn entries(&self) -> &[LearnedTagEntry] {
        &self.entries
    }

    /// Get mutable access to all entries.
    pub fn entries_mut(&mut self) -> &mut Vec<LearnedTagEntry> {
        &mut self.entries
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a new entry with today's date.
    pub fn new_entry(
        metric: StandardMetric,
        taxonomy: String,
        tag: String,
        unit: String,
        label: Option<String>,
        confidence: String,
        source: String,
        source_entity: String,
        approved: bool,
    ) -> LearnedTagEntry {
        LearnedTagEntry {
            metric,
            taxonomy,
            tag,
            unit,
            label,
            confidence,
            source,
            source_entity,
            discovered_at: Utc::now().format("%Y-%m-%d").to_string(),
            approved,
            category: None,
            review_reason: None,
        }
    }
}

/// Augment metric definitions with approved learned tag entries.
///
/// Primary tags (no category) are prepended to the `FirstMatch` chain for highest
/// priority. Categorized tags (subcomponents, disclosure items, etc.) are appended
/// as lower-priority fallbacks. Garbage entries are skipped entirely.
pub fn augment_definitions(
    definitions: &mut Vec<MetricDefinition>,
    entries: &[LearnedTagEntry],
) {
    // First pass: primary tags (no category) — prepend for highest priority
    for entry in entries.iter().filter(|e| e.approved && e.category.is_none()) {
        let spec = make_tag_spec(entry);
        add_to_definitions(definitions, &entry.metric, spec, true);
    }

    // Second pass: categorized tags — append as fallbacks
    for entry in entries
        .iter()
        .filter(|e| e.approved && e.category.is_some() && e.category.as_deref() != Some("garbage"))
    {
        let spec = make_tag_spec(entry);
        add_to_definitions(definitions, &entry.metric, spec, false);
    }
}

fn make_tag_spec(entry: &LearnedTagEntry) -> TagSpec {
    TagSpec {
        taxonomy: entry.taxonomy.clone(),
        tag: entry.tag.clone(),
        unit: if entry.unit == "unknown" {
            None
        } else {
            Some(entry.unit.clone())
        },
    }
}

fn add_to_definitions(
    definitions: &mut Vec<MetricDefinition>,
    metric: &StandardMetric,
    spec: TagSpec,
    prepend: bool,
) {
    if let Some(def) = definitions.iter_mut().find(|d| &d.metric == metric) {
        match &mut def.resolution {
            MetricResolution::FirstMatch(specs) => {
                if prepend {
                    specs.insert(0, spec);
                } else {
                    specs.push(spec);
                }
            }
            _ => {
                let existing_specs =
                    def.resolution.tag_specs().into_iter().cloned().collect::<Vec<_>>();
                let new_specs = if prepend {
                    let mut v = vec![spec];
                    v.extend(existing_specs);
                    v
                } else {
                    let mut v = existing_specs;
                    v.push(spec);
                    v
                };
                def.resolution = MetricResolution::FirstMatch(new_specs);
            }
        }
    } else {
        definitions.push(MetricDefinition {
            metric: metric.clone(),
            resolution: MetricResolution::FirstMatch(vec![spec]),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_augment_prepends_to_first_match() {
        let mut defs = vec![MetricDefinition {
            metric: StandardMetric::ResearchAndDevelopment,
            resolution: MetricResolution::FirstMatch(vec![TagSpec::gaap(
                "ResearchAndDevelopmentExpense",
            )]),
        }];

        let entries = vec![LearnedTagEntry {
            metric: StandardMetric::ResearchAndDevelopment,
            taxonomy: "us-gaap".to_string(),
            tag: "TechnologyAndContentExpense".to_string(),
            unit: "USD".to_string(),
            label: Some("Technology and content".to_string()),
            confidence: "high".to_string(),
            source: "llm".to_string(),
            source_entity: "AMAZON.COM, INC.".to_string(),
            discovered_at: "2026-02-08".to_string(),
            approved: true,
            category: None,
            review_reason: None,
        }];

        augment_definitions(&mut defs, &entries);

        assert_eq!(defs.len(), 1);
        match &defs[0].resolution {
            MetricResolution::FirstMatch(specs) => {
                assert_eq!(specs.len(), 2);
                assert_eq!(specs[0].tag, "TechnologyAndContentExpense");
                assert_eq!(specs[1].tag, "ResearchAndDevelopmentExpense");
            }
            _ => panic!("Expected FirstMatch"),
        }
    }

    #[test]
    fn test_augment_skips_unapproved() {
        let mut defs = vec![MetricDefinition {
            metric: StandardMetric::Revenue,
            resolution: MetricResolution::FirstMatch(vec![TagSpec::gaap("Revenues")]),
        }];

        let entries = vec![LearnedTagEntry {
            metric: StandardMetric::Revenue,
            taxonomy: "us-gaap".to_string(),
            tag: "SomeOtherTag".to_string(),
            unit: "USD".to_string(),
            label: None,
            confidence: "medium".to_string(),
            source: "keyword".to_string(),
            source_entity: "TEST CO".to_string(),
            discovered_at: "2026-02-08".to_string(),
            approved: false,
            category: None,
            review_reason: None,
        }];

        augment_definitions(&mut defs, &entries);

        match &defs[0].resolution {
            MetricResolution::FirstMatch(specs) => {
                assert_eq!(specs.len(), 1);
                assert_eq!(specs[0].tag, "Revenues");
            }
            _ => panic!("Expected FirstMatch"),
        }
    }

    #[test]
    fn test_augment_creates_new_definition() {
        let mut defs = Vec::new();

        let entries = vec![LearnedTagEntry {
            metric: StandardMetric::ResearchAndDevelopment,
            taxonomy: "us-gaap".to_string(),
            tag: "TechnologyAndContentExpense".to_string(),
            unit: "USD".to_string(),
            label: None,
            confidence: "high".to_string(),
            source: "llm".to_string(),
            source_entity: "TEST CO".to_string(),
            discovered_at: "2026-02-08".to_string(),
            approved: true,
            category: None,
            review_reason: None,
        }];

        augment_definitions(&mut defs, &entries);

        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].metric, StandardMetric::ResearchAndDevelopment);
    }

    #[test]
    fn test_store_dedup() {
        let mut store = LearnedTagStore {
            path: PathBuf::from("/tmp/test.json"),
            entries: Vec::new(),
        };

        let entry = LearnedTagEntry {
            metric: StandardMetric::Revenue,
            taxonomy: "us-gaap".to_string(),
            tag: "Revenues".to_string(),
            unit: "USD".to_string(),
            label: None,
            confidence: "high".to_string(),
            source: "llm".to_string(),
            source_entity: "TEST".to_string(),
            discovered_at: "2026-02-08".to_string(),
            approved: true,
            category: None,
            review_reason: None,
        };

        store.add(entry.clone());
        store.add(entry);

        assert_eq!(store.entries().len(), 1);
    }

    #[test]
    fn test_store_load_missing_file() {
        let store = LearnedTagStore::load("/tmp/nonexistent_learned_tags_test.json").unwrap();
        assert!(store.entries().is_empty());
    }
}
