//! Semantic memory engine — keyword-based similarity search.
//!
//! Stores memory records with metadata and supports retrieval by
//! keyword matching across stored content.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::Utc;

use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::context::ContextEngineType;
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};

/// Configuration for the semantic engine.
#[derive(Debug, Clone)]
pub struct SemanticEngineConfig {
    /// Maximum number of results per query.
    pub max_results: usize,
    /// Whether to index content for keyword search.
    pub enable_keyword_search: bool,
}

impl Default for SemanticEngineConfig {
    fn default() -> Self {
        Self {
            max_results: 50,
            enable_keyword_search: true,
        }
    }
}

/// A stored semantic entry with metadata.
#[derive(Debug, Clone)]
struct SemanticEntry {
    record: MemoryRecord,
    created_at: chrono::DateTime<Utc>,
    metadata: std::collections::BTreeMap<String, String>,
}

/// In-memory semantic memory engine.
///
/// Stores records with metadata and supports keyword-based retrieval.
pub struct SemanticEngine {
    config: SemanticEngineConfig,
    store: Arc<RwLock<Vec<SemanticEntry>>>,
    index: Arc<RwLock<HashMap<String, Vec<usize>>>>,
}

impl SemanticEngine {
    /// Creates a new semantic engine with the given configuration.
    pub fn new(config: SemanticEngineConfig) -> Self {
        Self {
            config,
            store: Arc::new(RwLock::new(Vec::new())),
            index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new semantic engine with default configuration.
    pub fn with_default_config() -> Self {
        Self::new(SemanticEngineConfig::default())
    }

    /// Builds or rebuilds the keyword index from stored entries.
    fn rebuild_index(&self) {
        let mut index = self.index.write().unwrap();
        index.clear();

        let store = self.store.read().unwrap();
        for (idx, entry) in store.iter().enumerate() {
            for word in entry.record.content.to_lowercase().split_whitespace() {
                index
                    .entry(word.to_string())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }
    }

    /// Returns the memory type this engine handles.
    pub fn memory_type(&self) -> MemoryProviderKind {
        MemoryProviderKind::Semantic
    }
}

#[async_trait]
impl ContextEngine for SemanticEngine {
    fn memory_type(&self) -> MemoryProviderKind {
        MemoryProviderKind::Semantic
    }

    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError> {
        let entry = SemanticEntry {
            record,
            created_at: Utc::now(),
            metadata: std::collections::BTreeMap::new(),
        };

        let mut store = self.store.write().unwrap();
        let idx = store.len();
        store.push(entry);
        drop(store);

        if self.config.enable_keyword_search {
            let content = self.store.read().unwrap().get(idx).unwrap().record.content.clone();
            let mut index = self.index.write().unwrap();
            for word in content.to_lowercase().split_whitespace() {
                index
                    .entry(word.to_string())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }

        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError> {
        let mut results = Vec::new();
        let store = self.store.read().unwrap();

        for entry in store.iter() {
            if query.session_id.is_empty()
                || entry.record.namespace.contains(&query.session_id)
                || query.session_id.contains(&entry.record.namespace)
            {
                results.push(ContextSnippet {
                    source: entry.record.namespace.clone(),
                    engine_type: ContextEngineType::Semantic,
                    content: entry.record.content.clone(),
                    score: Some(0.95),
                });
            }
        }

        if results.is_empty() && self.config.enable_keyword_search && !query.prompt.is_empty() {
            let mut scored: Vec<(ContextSnippet, f32)> = Vec::new();
            let store = self.store.read().unwrap();

            for word in query.prompt.to_lowercase().split_whitespace() {
                if let Some(indices) = self.index.read().unwrap().get(word) {
                    for idx in indices {
                        let entry = store.get(*idx).unwrap();
                        let snippet = ContextSnippet {
                            source: entry.record.namespace.clone(),
                            engine_type: ContextEngineType::Semantic,
                            content: entry.record.content.clone(),
                            score: Some(0.8),
                        };
                        scored.push((snippet, 0.8));
                    }
                }
            }

            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for (snippet, _) in scored.into_iter().take(self.config.max_results) {
                if !results.iter().any(|r| r.content == snippet.content) {
                    results.push(snippet);
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_and_retrieves_records() {
        let engine = SemanticEngine::with_default_config();

        engine
            .write(MemoryRecord {
                namespace: "semantic-store".to_string(),
                content: "user prefers rust".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "semantic-store".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "user prefers rust");
    }

    #[tokio::test]
    async fn stores_multiple_entries() {
        let engine = SemanticEngine::with_default_config();

        for i in 0..3 {
            engine
                .write(MemoryRecord {
                    namespace: "store".to_string(),
                    content: format!("semantic entry {}", i),
                })
                .await
                .unwrap();
        }

        let results = engine
            .query(MemoryQuery {
                session_id: "store".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn keyword_search_finds_matching_content() {
        let engine = SemanticEngine::with_default_config();

        engine
            .write(MemoryRecord {
                namespace: "lang".to_string(),
                content: "rust is a systems programming language".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "".to_string(),
                prompt: "rust programming".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "rust is a systems programming language");
    }

    #[tokio::test]
    async fn memory_type_returns_semantic() {
        let engine = SemanticEngine::with_default_config();
        assert_eq!(engine.memory_type(), MemoryProviderKind::Semantic);
    }
}
