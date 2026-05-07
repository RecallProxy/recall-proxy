//! Shared request-time context domain models used across crates.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// Request-time context assembly types (used by gateway orchestration)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextEngineType {
    Structural,
    Temporal,
    Semantic,
    Graph,
}

#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub tenant_id: String,
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub user_query: String,
    pub max_context_tokens: usize,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct EngineSnippet {
    pub engine_name: String,
    pub engine_type: ContextEngineType,
    pub rank: usize,
    pub text: String,
    pub relevance_score: Option<f32>,
    pub estimated_tokens: usize,
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EngineLookupMetrics {
    pub latency: Duration,
    pub timed_out: bool,
    pub failed: bool,
}

#[derive(Debug, Clone)]
pub struct EngineLookupResult {
    pub engine_name: String,
    pub snippets: Vec<EngineSnippet>,
    pub metrics: EngineLookupMetrics,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub total: usize,
    pub reserved_for_user_prompt: usize,
    pub reserved_for_system_prompt: usize,
}

impl TokenBudget {
    pub fn available_for_context(&self) -> usize {
        self.total
            .saturating_sub(self.reserved_for_user_prompt + self.reserved_for_system_prompt)
    }
}

#[derive(Debug, Clone)]
pub struct AssembledContext {
    pub synthesized_context: String,
    pub used_tokens: usize,
    pub dropped_snippets: usize,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Provider-facing context payload types (used by memory provider contracts)
// ---------------------------------------------------------------------------

/// A normalized context payload accepted across memory engines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextItem {
    pub id: String,
    pub content: String,
    pub metadata: BTreeMap<String, String>,
}

/// A retrievable context fragment returned from providers.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextChunk {
    pub item_id: String,
    pub content: String,
    pub score: Option<f32>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeWindow {
    pub start: SystemTime,
    pub end: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestRequest {
    pub tenant_id: String,
    pub session_id: Option<String>,
    pub correlation_id: String,
    pub occurred_at: SystemTime,
    pub items: Vec<ContextItem>,
    pub tags: BTreeMap<String, String>,
    pub deadline: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestAck {
    pub accepted: usize,
    pub rejected: usize,
    pub provider_request_id: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBudget {
    pub timeout: Duration,
    pub max_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRequest {
    pub tenant_id: String,
    pub session_id: Option<String>,
    pub correlation_id: String,
    pub query: String,
    pub top_k: usize,
    pub filters: BTreeMap<String, String>,
    pub time_window: Option<TimeWindow>,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryResponse {
    pub items: Vec<ContextChunk>,
    pub provider_latency_ms: u64,
    pub provider_request_id: Option<String>,
    pub partial: bool,
}
