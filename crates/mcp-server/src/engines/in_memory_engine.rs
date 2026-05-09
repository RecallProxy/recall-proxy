//! In-memory ContextEngine implementation.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};

pub struct InMemoryEngine {
    memory_type: MemoryProviderKind,
    data: Arc<RwLock<HashMap<String, Vec<MemoryRecord>>>>,
}

impl InMemoryEngine {
    pub fn new(memory_type: MemoryProviderKind) -> Self {
        Self {
            memory_type,
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn memory_type(&self) -> MemoryProviderKind {
        self.memory_type
    }

    pub fn insert(&self, record: MemoryRecord) {
        let mut data = self.data.write().unwrap();
        data.entry(record.namespace.clone())
            .or_insert_with(Vec::new)
            .push(record);
    }
}

#[async_trait]
impl ContextEngine for InMemoryEngine {
    fn memory_type(&self) -> MemoryProviderKind {
        self.memory_type
    }

    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError> {
        self.insert(record);
        Ok(())
    }

    async fn query(&self, _query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError> {
        let data = self.data.read().unwrap();
        let mut results = Vec::new();
        let engine_type = match self.memory_type {
            MemoryProviderKind::Semantic => recall_proxy_core::context::ContextEngineType::Semantic,
            MemoryProviderKind::Structural => recall_proxy_core::context::ContextEngineType::Structural,
            MemoryProviderKind::Temporal => recall_proxy_core::context::ContextEngineType::Temporal,
            MemoryProviderKind::Episodic => recall_proxy_core::context::ContextEngineType::Graph,
        };
        for (_namespace, records) in data.iter() {
            for record in records {
                results.push(ContextSnippet {
                    source: format!("{:?}", self.memory_type),
                    engine_type,
                    content: record.content.clone(),
                    score: Some(1.0),
                });
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn write_and_query_returns_record() {
        let engine = InMemoryEngine::new(MemoryProviderKind::Structural);
        engine
            .write(MemoryRecord {
                namespace: "ns-1".to_string(),
                content: "hello".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "ns-1".to_string(),
                prompt: "ns-1".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello");
    }
}
