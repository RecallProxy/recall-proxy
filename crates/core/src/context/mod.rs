//! Shared request-time context domain models used across crates.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Retrieval intent taxonomy
// ---------------------------------------------------------------------------

/// The kind of memory the caller is asking the gateway to retrieve.
///
/// This enum is the canonical way to express retrieval intent across the
/// RecallProxy data model. Each variant maps to one or more engine types
/// in the underlying provider system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalIntent {
    /// Retrieve episodic (raw transcript / turn-level) records.
    Episodic,
    /// Retrieve durable facts extracted from one or more episodes.
    Semantic,
    /// Retrieve time-ordered context (timelines, sessions, windows).
    Temporal,
    /// Retrieve structural / graph-based links (relationships, neighborhoods).
    Structural,
    /// Retrieve from all available engine types and merge results.
    Mixed,
}

impl Default for RetrievalIntent {
    fn default() -> Self {
        Self::Mixed
    }
}

// ---------------------------------------------------------------------------
// Memory artifact taxonomy
// ---------------------------------------------------------------------------

/// A normalized memory artifact that explicitly distinguishes the four
/// canonical memory categories used by RecallProxy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemoryArtifact {
    /// A raw episode captured during a session (unextracted transcript turn).
    Episodic {
        session_id: String,
        turn_id: String,
        speaker: String,
        content: String,
        observed_at: String,
        metadata: BTreeMap<String, String>,
    },
    /// A durable fact derived from one or more episodes (subject-predicate-object).
    Semantic {
        fact_id: String,
        session_id: String,
        subject: String,
        predicate: String,
        object: String,
        confidence: f32,
        source_turn_ids: Vec<String>,
        extracted_at: String,
    },
    /// A time-ordered context record (timeline entry, session window).
    Temporal {
        session_id: String,
        window_start: String,
        window_end: String,
        content: String,
        metadata: BTreeMap<String, String>,
    },
    /// A structural / graph-based link (relationship or neighborhood record).
    Structural {
        source_ref: String,
        target_ref: String,
        relation_type: String,
        weight: Option<f32>,
        metadata: BTreeMap<String, String>,
    },
}

impl MemoryArtifact {
    /// Return the canonical engine type this artifact belongs to.
    pub fn engine_type(&self) -> ContextEngineType {
        match self {
            MemoryArtifact::Episodic { .. } => ContextEngineType::Temporal,
            MemoryArtifact::Semantic { .. } => ContextEngineType::Semantic,
            MemoryArtifact::Temporal { .. } => ContextEngineType::Temporal,
            MemoryArtifact::Structural { .. } => ContextEngineType::Structural,
        }
    }

    /// Return the canonical artifact kind.
    pub fn kind(&self) -> MemoryArtifactKind {
        match self {
            MemoryArtifact::Episodic { .. } => MemoryArtifactKind::Episodic,
            MemoryArtifact::Semantic { .. } => MemoryArtifactKind::Semantic,
            MemoryArtifact::Temporal { .. } => MemoryArtifactKind::Temporal,
            MemoryArtifact::Structural { .. } => MemoryArtifactKind::Structural,
        }
    }
}

/// The canonical kind of a memory artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryArtifactKind {
    Episodic,
    Semantic,
    Temporal,
    Structural,
}

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
    pub retrieval_intent: RetrievalIntent,
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

// ---------------------------------------------------------------------------
// Tests for serialization and contract-level behavior
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_intent_serializes_to_snake_case() {
        let intents = [
            (RetrievalIntent::Episodic, "episodic"),
            (RetrievalIntent::Semantic, "semantic"),
            (RetrievalIntent::Temporal, "temporal"),
            (RetrievalIntent::Structural, "structural"),
            (RetrievalIntent::Mixed, "mixed"),
        ];

        for (intent, expected) in intents {
            let json = serde_json::to_string(&intent).expect("should serialize");
            assert_eq!(json, format!("\"{}\"", expected), "mismatch for {:?}", intent);
        }
    }

    #[test]
    fn retrieval_intent_deserializes_from_snake_case() {
        let yaml = r#"
- episodic
- semantic
- temporal
- structural
- mixed
"#;
        let intents: Vec<RetrievalIntent> =
            serde_yaml::from_str(yaml).expect("should deserialize");

        assert_eq!(intents.len(), 5);
        assert_eq!(intents[0], RetrievalIntent::Episodic);
        assert_eq!(intents[1], RetrievalIntent::Semantic);
        assert_eq!(intents[2], RetrievalIntent::Temporal);
        assert_eq!(intents[3], RetrievalIntent::Structural);
        assert_eq!(intents[4], RetrievalIntent::Mixed);
    }

    #[test]
    fn retrieval_intent_defaults_to_mixed() {
        let default_value: RetrievalIntent = Default::default();
        assert_eq!(default_value, RetrievalIntent::Mixed);
    }

    #[test]
    fn memory_artifact_episodic_round_trips() {
        let artifact = MemoryArtifact::Episodic {
            session_id: "sess-1".to_string(),
            turn_id: "turn-1".to_string(),
            speaker: "user".to_string(),
            content: "hello world".to_string(),
            observed_at: "2026-05-07T07:00:00Z".to_string(),
            metadata: BTreeMap::new(),
        };

        let json = serde_json::to_string(&artifact).expect("should serialize");
        let deserialized: MemoryArtifact = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(artifact, deserialized);
        assert!(json.contains(r#""kind":"episodic""#));
    }

    #[test]
    fn memory_artifact_semantic_round_trips() {
        let artifact = MemoryArtifact::Semantic {
            fact_id: "fact-1".to_string(),
            session_id: "sess-1".to_string(),
            subject: "user".to_string(),
            predicate: "lives_in".to_string(),
            object: "Berlin".to_string(),
            confidence: 0.95,
            source_turn_ids: vec!["turn-1".to_string()],
            extracted_at: "2026-05-07T07:01:00Z".to_string(),
        };

        let json = serde_json::to_string(&artifact).expect("should serialize");
        let deserialized: MemoryArtifact = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(artifact, deserialized);
        assert!(json.contains(r#""kind":"semantic""#));
    }

    #[test]
    fn memory_artifact_temporal_round_trips() {
        let artifact = MemoryArtifact::Temporal {
            session_id: "sess-1".to_string(),
            window_start: "2026-05-07T06:00:00Z".to_string(),
            window_end: "2026-05-07T08:00:00Z".to_string(),
            content: "timeline entry".to_string(),
            metadata: BTreeMap::new(),
        };

        let json = serde_json::to_string(&artifact).expect("should serialize");
        let deserialized: MemoryArtifact = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(artifact, deserialized);
        assert!(json.contains(r#""kind":"temporal""#));
    }

    #[test]
    fn memory_artifact_structural_round_trips() {
        let mut metadata = BTreeMap::new();
        metadata.insert("weight".to_string(), "0.8".to_string());

        let artifact = MemoryArtifact::Structural {
            source_ref: "user:alice".to_string(),
            target_ref: "user:bob".to_string(),
            relation_type: "knows".to_string(),
            weight: Some(0.8),
            metadata: metadata.clone(),
        };

        let json = serde_json::to_string(&artifact).expect("should serialize");
        let deserialized: MemoryArtifact = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(artifact, deserialized);
        assert!(json.contains(r#""kind":"structural""#));
    }

    #[test]
    fn memory_artifact_kind_maps_correctly() {
        let episodic = MemoryArtifact::Episodic {
            session_id: "s".to_string(),
            turn_id: "t".to_string(),
            speaker: "u".to_string(),
            content: "c".to_string(),
            observed_at: "2026-05-07T07:00:00Z".to_string(),
            metadata: BTreeMap::new(),
        };
        assert_eq!(episodic.kind(), MemoryArtifactKind::Episodic);

        let semantic = MemoryArtifact::Semantic {
            fact_id: "f".to_string(),
            session_id: "s".to_string(),
            subject: "s".to_string(),
            predicate: "p".to_string(),
            object: "o".to_string(),
            confidence: 0.5,
            source_turn_ids: vec![],
            extracted_at: "2026-05-07T07:00:00Z".to_string(),
        };
        assert_eq!(semantic.kind(), MemoryArtifactKind::Semantic);

        let temporal = MemoryArtifact::Temporal {
            session_id: "s".to_string(),
            window_start: "2026-05-07T06:00:00Z".to_string(),
            window_end: "2026-05-07T08:00:00Z".to_string(),
            content: "c".to_string(),
            metadata: BTreeMap::new(),
        };
        assert_eq!(temporal.kind(), MemoryArtifactKind::Temporal);

        let structural = MemoryArtifact::Structural {
            source_ref: "s".to_string(),
            target_ref: "t".to_string(),
            relation_type: "r".to_string(),
            weight: None,
            metadata: BTreeMap::new(),
        };
        assert_eq!(structural.kind(), MemoryArtifactKind::Structural);
    }

    #[test]
    fn memory_artifact_engine_type_mapping() {
        let episodic = MemoryArtifact::Episodic {
            session_id: "s".to_string(),
            turn_id: "t".to_string(),
            speaker: "u".to_string(),
            content: "c".to_string(),
            observed_at: "2026-05-07T07:00:00Z".to_string(),
            metadata: BTreeMap::new(),
        };
        assert_eq!(episodic.engine_type(), ContextEngineType::Temporal);

        let semantic = MemoryArtifact::Semantic {
            fact_id: "f".to_string(),
            session_id: "s".to_string(),
            subject: "s".to_string(),
            predicate: "p".to_string(),
            object: "o".to_string(),
            confidence: 0.5,
            source_turn_ids: vec![],
            extracted_at: "2026-05-07T07:00:00Z".to_string(),
        };
        assert_eq!(semantic.engine_type(), ContextEngineType::Semantic);

        let temporal = MemoryArtifact::Temporal {
            session_id: "s".to_string(),
            window_start: "2026-05-07T06:00:00Z".to_string(),
            window_end: "2026-05-07T08:00:00Z".to_string(),
            content: "c".to_string(),
            metadata: BTreeMap::new(),
        };
        assert_eq!(temporal.engine_type(), ContextEngineType::Temporal);

        let structural = MemoryArtifact::Structural {
            source_ref: "s".to_string(),
            target_ref: "t".to_string(),
            relation_type: "r".to_string(),
            weight: None,
            metadata: BTreeMap::new(),
        };
        assert_eq!(structural.engine_type(), ContextEngineType::Structural);
    }

    #[test]
    fn token_budget_available_calculation() {
        let budget = TokenBudget {
            total: 1000,
            reserved_for_user_prompt: 100,
            reserved_for_system_prompt: 200,
        };

        assert_eq!(budget.available_for_context(), 700);
    }

    #[test]
    fn token_budget_saturates_at_zero() {
        let budget = TokenBudget {
            total: 100,
            reserved_for_user_prompt: 60,
            reserved_for_system_prompt: 60,
        };

        assert_eq!(budget.available_for_context(), 0);
    }
}
