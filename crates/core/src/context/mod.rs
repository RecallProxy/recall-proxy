//! Shared request-time context domain models used across crates.

use std::collections::HashMap;
use std::time::Duration;

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
