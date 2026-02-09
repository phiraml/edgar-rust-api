pub mod cache;
pub mod http;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::api;
use crate::error::{EdgarError, Result};
use crate::models::cik::Cik;
use crate::models::company::Company;
use crate::models::company_concept::CompanyConceptResponse;
use crate::models::company_facts::CompanyFactsResponse;
use crate::models::filing::Filing;
use crate::models::filing_index::IndexEntry;
use crate::models::frame::FrameResponse;
use crate::models::period::CalendarPeriod;
use crate::models::search::SearchResponse;
use crate::models::ticker::TickerMap;
use crate::standardizer::catalog::sector::sector_definitions;
use crate::standardizer::catalog::{DefaultCatalog, MetricCatalog};
use crate::standardizer::coverage::{CoverageAnalyzer, CoverageReport};
use crate::standardizer::engine::StandardizationEngine;
use crate::standardizer::learned_tags::{self, LearnedTagStore};
use crate::standardizer::llm_classifier::{LlmClassifier, classification_to_candidate};
use crate::standardizer::output::StandardizedFinancials;

use self::cache::EdgarCache;
use self::http::RateLimitedHttp;

const EDGAR_BASE: &str = "https://data.sec.gov";
const EDGAR_EFTS: &str = "https://efts.sec.gov";
const EDGAR_WWW: &str = "https://www.sec.gov";

/// Builder for configuring an `EdgarClient`.
pub struct EdgarClientBuilder {
    user_agent: String,
    requests_per_second: Option<u32>,
    cache_capacity: u64,
    anthropic_api_key: Option<String>,
    tag_store_path: Option<PathBuf>,
}

impl EdgarClientBuilder {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self {
            user_agent: user_agent.into(),
            requests_per_second: None,
            cache_capacity: 1000,
            anthropic_api_key: None,
            tag_store_path: None,
        }
    }

    /// Set the maximum requests per second (default: 10).
    pub fn requests_per_second(mut self, rps: u32) -> Self {
        self.requests_per_second = Some(rps);
        self
    }

    /// Set the maximum number of cached entries (default: 1000).
    pub fn cache_capacity(mut self, capacity: u64) -> Self {
        self.cache_capacity = capacity;
        self
    }

    /// Set an Anthropic API key for LLM-enhanced coverage gap analysis.
    pub fn anthropic_api_key(mut self, key: impl Into<String>) -> Self {
        self.anthropic_api_key = Some(key.into());
        self
    }

    /// Set the path for the persistent learned tag store (JSON file).
    ///
    /// Discovered tags will be saved here and loaded on future runs to
    /// augment the standardizer's tag resolution.
    pub fn tag_store_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.tag_store_path = Some(path.into());
        self
    }

    /// Build the `EdgarClient`.
    pub fn build(self) -> Result<EdgarClient> {
        let http = RateLimitedHttp::new(&self.user_agent, self.requests_per_second)?;
        let cache = EdgarCache::new(self.cache_capacity);
        let ticker_map = Arc::new(RwLock::new(None));

        Ok(EdgarClient {
            http,
            cache,
            ticker_map,
            anthropic_api_key: self.anthropic_api_key,
            tag_store_path: self.tag_store_path,
        })
    }
}

/// The primary entry point for interacting with SEC EDGAR APIs.
///
/// Use `EdgarClient::builder("your-email@example.com")` to create an instance.
#[derive(Clone)]
pub struct EdgarClient {
    pub(crate) http: RateLimitedHttp,
    pub(crate) cache: EdgarCache,
    pub(crate) ticker_map: Arc<RwLock<Option<TickerMap>>>,
    pub(crate) anthropic_api_key: Option<String>,
    pub(crate) tag_store_path: Option<PathBuf>,
}

impl EdgarClient {
    /// Create a builder for `EdgarClient`.
    ///
    /// `user_agent` should be your email or app identifier per SEC requirements.
    pub fn builder(user_agent: impl Into<String>) -> EdgarClientBuilder {
        EdgarClientBuilder::new(user_agent)
    }

    /// Create a client with default settings.
    pub fn new(user_agent: impl Into<String>) -> Result<Self> {
        Self::builder(user_agent).build()
    }

    // ─── Ticker Resolution ────────────────────────────────────────────

    /// Load and cache the ticker map from SEC.
    pub async fn load_ticker_map(&self) -> Result<TickerMap> {
        {
            let guard = self.ticker_map.read().await;
            if let Some(ref map) = *guard {
                return Ok(map.clone());
            }
        }

        let map = api::tickers::fetch_ticker_map(&self.http).await?;
        {
            let mut guard = self.ticker_map.write().await;
            *guard = Some(map.clone());
        }
        Ok(map)
    }

    /// Resolve a ticker symbol to a CIK.
    pub async fn resolve_ticker(&self, ticker: &str) -> Result<Cik> {
        let map = self.load_ticker_map().await?;
        map.lookup_ticker(ticker)
            .map(|ct| ct.cik_str)
            .ok_or_else(|| EdgarError::TickerNotFound(ticker.to_string()))
    }

    /// Resolve a ticker-or-CIK string to a CIK.
    pub async fn resolve(&self, identifier: &str) -> Result<Cik> {
        // Try parsing as CIK first
        if let Ok(cik) = identifier.parse::<Cik>() {
            return Ok(cik);
        }
        // Otherwise treat as ticker
        self.resolve_ticker(identifier).await
    }

    // ─── Core API Methods ─────────────────────────────────────────────

    /// Get company metadata and recent filings.
    pub async fn company(&self, identifier: &str) -> Result<Company> {
        let cik = self.resolve(identifier).await?;
        let resp = api::submissions::fetch_submissions(&self.http, &self.cache, cik).await?;
        Ok(resp.to_company())
    }

    /// Get recent filings for a company.
    pub async fn filings(&self, identifier: &str) -> Result<Vec<Filing>> {
        let cik = self.resolve(identifier).await?;
        api::submissions::fetch_filings(&self.http, &self.cache, cik).await
    }

    /// Get all XBRL facts for a company.
    pub async fn company_facts(&self, identifier: &str) -> Result<CompanyFactsResponse> {
        let cik = self.resolve(identifier).await?;
        api::company_facts::fetch_company_facts(&self.http, &self.cache, cik).await
    }

    /// Get a specific XBRL concept for a company.
    pub async fn company_concept(
        &self,
        identifier: &str,
        taxonomy: &str,
        tag: &str,
    ) -> Result<CompanyConceptResponse> {
        let cik = self.resolve(identifier).await?;
        api::company_concept::fetch_company_concept(&self.http, &self.cache, cik, taxonomy, tag)
            .await
    }

    /// Get cross-company XBRL frame data for a specific period.
    pub async fn frame(
        &self,
        taxonomy: &str,
        tag: &str,
        unit: &str,
        period: &CalendarPeriod,
    ) -> Result<FrameResponse> {
        api::frames::fetch_frame(&self.http, &self.cache, taxonomy, tag, unit, period).await
    }

    /// Full-text search over EDGAR filings.
    pub async fn search(
        &self,
        query: &str,
        forms: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        start: u32,
    ) -> Result<SearchResponse> {
        api::search::efts_search(
            &self.http, &self.cache, query, forms, start_date, end_date, start,
        )
        .await
    }

    /// Fetch full-index entries for a given year/quarter.
    pub async fn full_index(
        &self,
        year: i32,
        quarter: u8,
    ) -> Result<Vec<IndexEntry>> {
        api::index::fetch_full_index(&self.http, year, quarter).await
    }

    // ─── Standardizer ─────────────────────────────────────────────────

    /// Build augmented metric definitions by loading learned tags and merging with defaults.
    fn build_augmented_definitions(
        &self,
        company: &Company,
    ) -> Result<Option<Vec<crate::standardizer::catalog::MetricDefinition>>> {
        if let Some(ref path) = self.tag_store_path {
            let store = LearnedTagStore::load(path)?;
            if !store.entries().is_empty() {
                let catalog = DefaultCatalog;
                let mut definitions = catalog.definitions();
                definitions.extend(sector_definitions(company.sic.as_deref()));
                learned_tags::augment_definitions(&mut definitions, store.entries());
                return Ok(Some(definitions));
            }
        }
        Ok(None)
    }

    /// Get standardized financials for a company with automatic XBRL tag resolution.
    pub async fn financials(&self, identifier: &str) -> Result<StandardizedFinancials> {
        let facts = self.company_facts(identifier).await?;
        let company = self.company(identifier).await?;

        let engine = match self.build_augmented_definitions(&company)? {
            Some(defs) => {
                let catalog = crate::standardizer::catalog::VecCatalog(defs);
                StandardizationEngine::with_catalog(Box::new(catalog))
            }
            None => StandardizationEngine::new(),
        };

        engine.standardize(&facts, &company)
    }

    // ─── Coverage Gap Analysis ──────────────────────────────────────────

    /// Analyze coverage gaps in standardized financials for a company.
    ///
    /// Uses keyword heuristics to find candidate tags for missing metrics.
    /// If an Anthropic API key was provided via the builder, also uses Claude
    /// to classify ambiguous tags that keywords miss.
    ///
    /// When a `tag_store_path` is configured, discovered tags are persisted to
    /// a JSON file and loaded on future runs to augment tag resolution.
    pub async fn coverage_gaps(&self, identifier: &str) -> Result<CoverageReport> {
        let facts = self.company_facts(identifier).await?;
        let company = self.company(identifier).await?;

        // Load learned tags store and build augmented definitions
        let mut store = self
            .tag_store_path
            .as_ref()
            .map(|p| LearnedTagStore::load(p))
            .transpose()?;

        let augmented_defs = {
            let catalog = DefaultCatalog;
            let mut defs = catalog.definitions();
            defs.extend(sector_definitions(company.sic.as_deref()));
            if let Some(ref store) = store {
                learned_tags::augment_definitions(&mut defs, store.entries());
            }
            defs
        };

        // Standardize with augmented definitions
        let engine = {
            let catalog = crate::standardizer::catalog::VecCatalog(augmented_defs.clone());
            StandardizationEngine::with_catalog(Box::new(catalog))
        };
        let financials = engine.standardize(&facts, &company)?;

        // Step 1: keyword analysis (using augmented definitions)
        let mut report =
            CoverageAnalyzer::analyze(&facts, &company, &financials, Some(&augmented_defs));

        // Step 2: LLM classification (if API key provided & missing metrics with no candidates)
        if let Some(ref api_key) = self.anthropic_api_key {
            let needs_llm: Vec<_> = report
                .missing_metrics
                .iter()
                .filter(|m| m.candidates.is_empty())
                .collect();

            if !needs_llm.is_empty() {
                // Collect all unmatched tags from the company's facts
                let known_tags: std::collections::HashSet<String> = augmented_defs
                    .iter()
                    .flat_map(|d| {
                        d.resolution
                            .tag_specs()
                            .into_iter()
                            .map(|s| format!("{}:{}", s.taxonomy, s.tag))
                    })
                    .collect();

                let mut unmatched_tags = Vec::new();
                for (taxonomy, tags) in &facts.facts {
                    for (tag, tag_data) in tags {
                        let key = format!("{taxonomy}:{tag}");
                        if known_tags.contains(&key) {
                            continue;
                        }
                        // Get latest annual value
                        let mut latest_value = None;
                        let mut best_end = String::new();
                        for values in tag_data.units.values() {
                            for fact in values {
                                if fact.is_annual() {
                                    if let Some(val) = fact.val {
                                        if fact.end > best_end {
                                            best_end = fact.end.clone();
                                            latest_value = Some(val);
                                        }
                                    }
                                }
                            }
                        }
                        unmatched_tags.push((
                            taxonomy.clone(),
                            tag.clone(),
                            tag_data.label.clone(),
                            latest_value,
                        ));
                    }
                }

                let classifier = LlmClassifier::new(api_key.clone());
                let needs_llm_refs: Vec<_> = report
                    .missing_metrics
                    .iter()
                    .filter(|m| m.candidates.is_empty())
                    .collect();

                match classifier
                    .classify(&report.entity_name, &needs_llm_refs, &unmatched_tags)
                    .await
                {
                    Ok(classifications) => {
                        for cls in &classifications {
                            if let Some(candidate) =
                                classification_to_candidate(cls, &unmatched_tags)
                            {
                                // Find the matching missing metric and add the candidate
                                for mm in &mut report.missing_metrics {
                                    let metric_name = format!("{:?}", mm.metric);
                                    if metric_name == cls.metric {
                                        mm.candidates.push(candidate.clone());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("LLM classification failed: {e}");
                    }
                }
            }
        }

        // Step 3: Save discoveries to the learned tag store
        if let Some(ref mut store) = store {
            let entity_name = report.entity_name.clone();

            for mm in &report.missing_metrics {
                for candidate in &mm.candidates {
                    let is_llm = candidate.match_reason.starts_with("llm:");
                    let is_top_keyword = !is_llm
                        && candidate.latest_value.is_some()
                        && mm.candidates.iter().position(|c| std::ptr::eq(c, candidate)) == Some(0);

                    if is_llm {
                        let confidence = if candidate.match_reason.contains("confidence: high") {
                            "high"
                        } else if candidate.match_reason.contains("confidence: medium") {
                            "medium"
                        } else {
                            "low"
                        };
                        let approved = confidence == "high";

                        store.add(LearnedTagStore::new_entry(
                            mm.metric.clone(),
                            candidate.taxonomy.clone(),
                            candidate.tag.clone(),
                            candidate.unit.clone(),
                            candidate.label.clone(),
                            confidence.to_string(),
                            "llm".to_string(),
                            entity_name.clone(),
                            approved,
                        ));
                    } else if is_top_keyword {
                        store.add(LearnedTagStore::new_entry(
                            mm.metric.clone(),
                            candidate.taxonomy.clone(),
                            candidate.tag.clone(),
                            candidate.unit.clone(),
                            candidate.label.clone(),
                            "keyword".to_string(),
                            "keyword".to_string(),
                            entity_name.clone(),
                            false,
                        ));
                    }
                }
            }

            store.save()?;
        }

        Ok(report)
    }

    // ─── Watcher ───────────────────────────────────────────────────────

    /// Start a filing watcher with the given configuration.
    pub fn start_watcher(
        &self,
        config: crate::watcher::WatcherConfig,
    ) -> crate::watcher::WatcherHandle {
        crate::watcher::start_watcher(self.http.clone(), config)
    }

    // ─── Convenience ──────────────────────────────────────────────────

    pub fn base_url() -> &'static str {
        EDGAR_BASE
    }

    pub fn efts_url() -> &'static str {
        EDGAR_EFTS
    }

    pub fn www_url() -> &'static str {
        EDGAR_WWW
    }
}
