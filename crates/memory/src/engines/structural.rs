//! Structural memory engine — relationship and fact storage.
//!
//! Stores structured facts and relationships between entities,
//! supporting graph-like traversal queries.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::Utc;

use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::context::ContextEngineType;
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};

/// Configuration for the structural engine.
#[derive(Debug, Clone)]
pub struct StructuralEngineConfig {
    /// Maximum number of facts to store.
    pub max_facts: usize,
    /// Whether to enable relationship extraction.
    pub enable_relationships: bool,
}

impl Default for StructuralEngineConfig {
    fn default() -> Self {
        Self {
            max_facts: 50000,
            enable_relationships: true,
        }
    }
}

/// A stored fact with subject-predicate-object structure.
#[derive(Debug, Clone)]
struct FactEntry {
    subject: String,
    predicate: String,
    object: String,
    confidence: f32,
    source_namespace: String,
    created_at: chrono::DateTime<Utc>,
}

/// In-memory structural memory engine.
///
/// Stores structured facts and relationships with graph-like query support.
pub struct StructuralEngine {
    config: StructuralEngineConfig,
    facts: Arc<RwLock<Vec<FactEntry>>>,
    /// Index: subject -> list of fact indices
    subject_index: Arc<RwLock<HashMap<String, Vec<usize>>>>,
    /// Index: predicate -> list of fact indices
    predicate_index: Arc<RwLock<HashMap<String, Vec<usize>>>>,
}

impl StructuralEngine {
    /// Creates a new structural engine with the given configuration.
    pub fn new(config: StructuralEngineConfig) -> Self {
        Self {
            config,
            facts: Arc::new(RwLock::new(Vec::new())),
            subject_index: Arc::new(RwLock::new(HashMap::new())),
            predicate_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new structural engine with default configuration.
    pub fn with_default_config() -> Self {
        Self::new(StructuralEngineConfig::default())
    }

    /// Parses a content string into subject-predicate-object triples.
    fn parse_fact(content: &str) -> (String, String, String) {
        let parts: Vec<&str> = content.splitn(3, '|').collect();
        if parts.len() == 3 {
            (
                parts[0].trim().to_string(),
                parts[1].trim().to_string(),
                parts[2].trim().to_string(),
            )
        } else {
            (
                "entity".to_string(),
                "has_property".to_string(),
                content.to_string(),
            )
        }
    }

    /// Returns the memory type this engine handles.
    pub fn memory_type(&self) -> MemoryProviderKind {
        MemoryProviderKind::Structural
    }
}

#[async_trait]
impl ContextEngine for StructuralEngine {
    fn memory_type(&self) -> MemoryProviderKind {
        MemoryProviderKind::Structural
    }

    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError> {
        let (subject, predicate, object) = Self::parse_fact(&record.content);
        let confidence = if predicate == "has_property" { 0.7 } else { 0.95 };

        let fact = FactEntry {
            subject: subject.clone(),
            predicate: predicate.clone(),
            object: object.clone(),
            confidence,
            source_namespace: record.namespace.clone(),
            created_at: Utc::now(),
        };

        let mut facts = self.facts.write().unwrap();
        let idx = facts.len();

        if facts.len() >= self.config.max_facts {
            facts.remove(0);
            let mut si = self.subject_index.write().unwrap();
            let mut pi = self.predicate_index.write().unwrap();
            si.retain(|_, indices| {
                *indices = indices.iter().map(|i| i.saturating_sub(1)).collect::<Vec<_>>();
                !indices.is_empty()
            });
            pi.retain(|_, indices| {
                *indices = indices.iter().map(|i| i.saturating_sub(1)).collect::<Vec<_>>();
                !indices.is_empty()
            });
        }

        facts.push(fact);
        drop(facts);

        if self.config.enable_relationships {
            let mut si = self.subject_index.write().unwrap();
            si.entry(subject).or_insert_with(Vec::new).push(idx);

            let mut pi = self.predicate_index.write().unwrap();
            pi.entry(predicate).or_insert_with(Vec::new).push(idx);
        }

        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError> {
        let mut results = Vec::new();
        let facts = self.facts.read().unwrap();

        for fact in facts.iter().rev() {
            if query.session_id.is_empty()
                || fact.source_namespace.contains(&query.session_id)
                || query.session_id.contains(&fact.source_namespace)
            {
                results.push(ContextSnippet {
                    source: fact.source_namespace.clone(),
                    engine_type: ContextEngineType::Structural,
                    content: format!("{} | {} | {}", fact.subject, fact.predicate, fact.object),
                    score: Some(fact.confidence),
                });
            }

            if results.len() >= query.max_results {
                break;
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_and_retrieves_facts() {
        let engine = StructuralEngine::with_default_config();

        engine
            .write(MemoryRecord {
                namespace: "facts-1".to_string(),
                content: "user|lives_in|Berlin".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "facts-1".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "user | lives_in | Berlin");
    }

    #[tokio::test]
    async fn parses_facts_from_content() {
        let engine = StructuralEngine::with_default_config();

        engine
            .write(MemoryRecord {
                namespace: "relations".to_string(),
                content: "Alice|knows|Bob".to_string(),
            })
            .await
            .unwrap();

        engine
            .write(MemoryRecord {
                namespace: "relations".to_string(),
                content: "Bob|works_at|Acme".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "relations".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn handles_non_pipe_delimited_content() {
        let engine = StructuralEngine::with_default_config();

        engine
            .write(MemoryRecord {
                namespace: "freeform".to_string(),
                content: "freeform text content".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "freeform".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "entity | has_property | freeform text content");
    }

    #[tokio::test]
    async fn memory_type_returns_structural() {
        let engine = StructuralEngine::with_default_config();
        assert_eq!(engine.memory_type(), MemoryProviderKind::Structural);
    }
}
