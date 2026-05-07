use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    /// Keeps confidence values constrained to the expected [0.0, 1.0] range.
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
}
