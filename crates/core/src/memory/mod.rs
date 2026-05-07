use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::context::{ContextChunk, IngestAck, IngestRequest, QueryRequest, QueryResponse};
use crate::error::ProviderResult;

/// A normalized memory item produced or consumed by providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub namespace: String,
    pub content: String,
}

/// Raw message unit captured during ingest before any extraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTranscript {
    pub session_id: String,
    pub turn_id: String,
    pub speaker: TranscriptSpeaker,
    pub content: String,
    pub observed_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Canonical speaker roles for temporal memory ingestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptSpeaker {
    User,
    Assistant,
    Tool,
    System,
}

/// Fact extracted from one or more transcript turns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedFact {
    pub fact_id: String,
    pub session_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub source_turn_ids: Vec<String>,
    pub extracted_at: DateTime<Utc>,
}

impl DerivedFact {
    pub fn is_confidence_valid(&self) -> bool {
        (0.0..=1.0).contains(&self.confidence)
    }
}

/// Memory engine classes that RecallProxy can route to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryProviderKind {
    Semantic,
    Structural,
    Temporal,
}

/// Normalized payload boundary given to provider adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderWritePayload {
    pub provider: MemoryProviderKind,
    pub session_id: String,
    pub dedupe_key: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub payload: ProviderWriteBody,
}

/// Provider payload shape, constrained by memory class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderWriteBody {
    Temporal { transcript: RawTranscript },
    Structural { facts: Vec<DerivedFact> },
    Semantic {
        transcript: RawTranscript,
        facts: Vec<DerivedFact>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Semantic,
    Structural,
    Temporal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub kind: MemoryKind,
    pub supports_ingest: bool,
    pub supports_query: bool,
    pub supports_streaming: bool,
    pub max_batch_size: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub provider_id: Cow<'static, str>,
    pub version: Cow<'static, str>,
    pub capabilities: Vec<CapabilityDescriptor>,
}

/// Base contract expected from any context memory provider.
pub trait MemoryProvider: Send + Sync {
    fn metadata(&self) -> ProviderMetadata;
    async fn healthcheck(&self, timeout: Duration) -> ProviderResult<()>;
}

pub trait SemanticMemoryProvider: MemoryProvider {
    async fn ingest_semantic(&self, request: IngestRequest) -> ProviderResult<IngestAck>;
    async fn query_semantic(&self, request: QueryRequest) -> ProviderResult<QueryResponse>;
}

pub trait StructuralMemoryProvider: MemoryProvider {
    async fn ingest_structural(&self, request: IngestRequest) -> ProviderResult<IngestAck>;
    async fn query_structural(&self, request: QueryRequest) -> ProviderResult<QueryResponse>;
}

pub trait TemporalMemoryProvider: MemoryProvider {
    async fn ingest_temporal(&self, request: IngestRequest) -> ProviderResult<IngestAck>;
    async fn query_temporal(&self, request: QueryRequest) -> ProviderResult<QueryResponse>;

    async fn stream_temporal(&self, request: QueryRequest) -> ProviderResult<Vec<ContextChunk>> {
        self.query_temporal(request).await.map(|response| response.items)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn sample_transcript() -> RawTranscript {
        RawTranscript {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            speaker: TranscriptSpeaker::User,
            content: "I moved to Berlin in 2023.".to_string(),
            observed_at: Utc.with_ymd_and_hms(2026, 5, 7, 7, 0, 0).unwrap(),
            metadata: BTreeMap::new(),
        }
    }

    fn sample_fact(confidence: f32) -> DerivedFact {
        DerivedFact {
            fact_id: "fact-1".to_string(),
            session_id: "session-1".to_string(),
            subject: "user".to_string(),
            predicate: "lives_in".to_string(),
            object: "Berlin".to_string(),
            confidence,
            source_turn_ids: vec!["turn-1".to_string()],
            extracted_at: Utc.with_ymd_and_hms(2026, 5, 7, 7, 1, 0).unwrap(),
        }
    }

    #[test]
    fn derived_fact_confidence_accepts_normalized_range() {
        assert!(sample_fact(0.0).is_confidence_valid());
        assert!(sample_fact(1.0).is_confidence_valid());
        assert!(sample_fact(0.5).is_confidence_valid());
    }

    #[test]
    fn derived_fact_confidence_rejects_outside_range() {
        assert!(!sample_fact(-0.1).is_confidence_valid());
        assert!(!sample_fact(1.1).is_confidence_valid());
    }

    #[test]
    fn semantic_payload_can_carry_transcript_and_facts() {
        let payload = ProviderWritePayload {
            provider: MemoryProviderKind::Semantic,
            session_id: "session-1".to_string(),
            dedupe_key: Some("turn-1".to_string()),
            timestamp: Utc.with_ymd_and_hms(2026, 5, 7, 7, 2, 0).unwrap(),
            payload: ProviderWriteBody::Semantic {
                transcript: sample_transcript(),
                facts: vec![sample_fact(0.91)],
            },
        };

        assert_eq!(payload.provider, MemoryProviderKind::Semantic);
        match payload.payload {
            ProviderWriteBody::Semantic { facts, .. } => assert_eq!(facts.len(), 1),
            _ => panic!("expected semantic payload"),
        }
    }

    #[test]
    fn memory_record_holds_values() {
        let record = MemoryRecord {
            namespace: "session-1".to_string(),
            content: "hello world".to_string(),
        };

        assert_eq!(record.namespace, "session-1");
        assert_eq!(record.content, "hello world");
    }
}
