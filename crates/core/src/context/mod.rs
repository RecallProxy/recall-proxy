use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

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
