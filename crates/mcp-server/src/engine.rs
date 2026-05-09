//! In-memory engine implementations for testing the MCP server.

use async_trait::async_trait;
use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::context::ContextEngineType;
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub namespace: String,
    pub content: String,
}

pub struct InMemoryEngine {
    memory_type: MemoryProviderKind,
    store: Arc<RwLock<HashMap<String, Vec<MemoryEntry>>>>,
}

impl InMemoryEngine {
    pub fn new(memory_type: MemoryProviderKind) -> Self {
        Self {
            memory_type,
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn memory_type(&self) -> MemoryProviderKind {
        self.memory_type
    }

    pub fn store(&self) -> Arc<RwLock<HashMap<String, Vec<MemoryEntry>>>> {
        Arc::clone(&self.store)
    }
}

fn kind_to_engine_type(kind: MemoryProviderKind) -> ContextEngineType {
    match kind {
        MemoryProviderKind::Semantic => ContextEngineType::Semantic,
        MemoryProviderKind::Structural => ContextEngineType::Structural,
        MemoryProviderKind::Temporal => ContextEngineType::Temporal,
        MemoryProviderKind::Episodic => ContextEngineType::Graph,
    }
}

#[async_trait]
impl ContextEngine for InMemoryEngine {
    fn memory_type(&self) -> MemoryProviderKind {
        self.memory_type
    }

    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError> {
        let mut store = self.store.write().unwrap();
        let namespace = record.namespace.clone();
        store
            .entry(namespace)
            .or_insert_with(Vec::new)
            .push(MemoryEntry {
                namespace: record.namespace,
                content: record.content,
            });
        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError> {
        let store = self.store.read().unwrap();
        let mut results = Vec::new();
        let engine_type = kind_to_engine_type(self.memory_type());

        for (namespace, entries) in store.iter() {
            if query.session_id.contains(namespace) || namespace.contains(&query.session_id) {
                for entry in entries {
                    results.push(ContextSnippet {
                        source: namespace.clone(),
                        engine_type,
                        content: entry.content.clone(),
                        score: None,
                    });
                }
            }
        }

        if results.is_empty() {
            for (namespace, entries) in store.iter() {
                for entry in entries {
                    results.push(ContextSnippet {
                        source: namespace.clone(),
                        engine_type,
                        content: entry.content.clone(),
                        score: None,
                    });
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
    async fn in_memory_engine_writes_and_reads_records() {
        let engine = InMemoryEngine::new(MemoryProviderKind::Structural);
        assert_eq!(engine.memory_type(), MemoryProviderKind::Structural);

        engine
            .write(MemoryRecord {
                namespace: "test-ns".to_string(),
                content: "hello world".to_string(),
            })
            .await
            .expect("write should succeed");

        let results = engine
            .query(MemoryQuery {
                session_id: "test".to_string(),
                prompt: "hello".to_string(),
                max_results: 10,
            })
            .await
            .expect("query should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[tokio::test]
    async fn in_memory_engine_stores_multiple_entries() {
        let engine = InMemoryEngine::new(MemoryProviderKind::Temporal);

        for i in 0..3 {
            engine
                .write(MemoryRecord {
                    namespace: format!("ns-{}", i),
                    content: format!("content-{}", i),
                })
                .await
                .expect("write should succeed");
        }

        let results = engine
            .query(MemoryQuery {
                session_id: "x".to_string(),
                prompt: "x".to_string(),
                max_results: 10,
            })
            .await
            .expect("query should succeed");

        assert_eq!(results.len(), 3);
    }
}
