pub mod orchestrator;
pub mod response;

use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};
use std::collections::HashMap;
use std::sync::Arc;

/// Gateway runtime using per-engine-type traits (e.g. StructuralEngine, TemporalEngine).
/// See `orchestrator::ContextGateway` for the primary orchestrator implementation.

/// Generic gateway over a unified ContextEngine trait.
pub struct ContextMemoryGateway {
    engines: HashMap<MemoryProviderKind, Arc<dyn ContextEngine>>,
}

impl ContextMemoryGateway {
    pub fn new(engines: Vec<Arc<dyn ContextEngine>>) -> Self {
        let engines = engines
            .into_iter()
            .map(|engine| (engine.memory_type(), engine))
            .collect();
        Self { engines }
    }

    pub async fn ingest(
        &self,
        structural_record: MemoryRecord,
        temporal_record: MemoryRecord,
    ) -> Result<(), EngineError> {
        let structural_engine = self
            .engines
            .get(&MemoryProviderKind::Structural)
            .ok_or_else(|| EngineError::new("missing structural engine"))?;
        let temporal_engine = self
            .engines
            .get(&MemoryProviderKind::Temporal)
            .ok_or_else(|| EngineError::new("missing temporal engine"))?;

        let (structural_result, temporal_result) = tokio::join!(
            structural_engine.write(structural_record),
            temporal_engine.write(temporal_record)
        );

        structural_result?;
        temporal_result?;
        Ok(())
    }

    pub async fn assemble_context(
        &self,
        query: MemoryQuery,
    ) -> Result<Vec<ContextSnippet>, EngineError> {
        let mut contexts = Vec::new();

        for memory_type in [
            MemoryProviderKind::Semantic,
            MemoryProviderKind::Structural,
            MemoryProviderKind::Temporal,
        ] {
            if let Some(engine) = self.engines.get(&memory_type) {
                let mut result = engine.query(query.clone()).await?;
                contexts.append(&mut result);
            }
        }

        Ok(contexts)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use recall_proxy_core::engine::{ContextEngine, EngineError};
    use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
    use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};

    use crate::ContextMemoryGateway;

    struct InMemoryEngine {
        memory_type: MemoryProviderKind,
        writes: Arc<Mutex<Vec<MemoryRecord>>>,
        query_results: Vec<ContextSnippet>,
    }

    #[async_trait]
    impl ContextEngine for InMemoryEngine {
        fn memory_type(&self) -> MemoryProviderKind {
            self.memory_type
        }

        async fn write(&self, record: MemoryRecord) -> Result<(), EngineError> {
            self.writes
                .lock()
                .expect("lock should not be poisoned")
                .push(record);
            Ok(())
        }

        async fn query(&self, _query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError> {
            Ok(self.query_results.clone())
        }
    }

    #[tokio::test]
    async fn gateway_ingest_routes_to_structural_and_temporal_engines() {
        let structural_writes = Arc::new(Mutex::new(Vec::new()));
        let temporal_writes = Arc::new(Mutex::new(Vec::new()));

        let structural = Arc::new(InMemoryEngine {
            memory_type: MemoryProviderKind::Structural,
            writes: Arc::clone(&structural_writes),
            query_results: vec![],
        });
        let temporal = Arc::new(InMemoryEngine {
            memory_type: MemoryProviderKind::Temporal,
            writes: Arc::clone(&temporal_writes),
            query_results: vec![],
        });
        let semantic = Arc::new(InMemoryEngine {
            memory_type: MemoryProviderKind::Semantic,
            writes: Arc::new(Mutex::new(Vec::new())),
            query_results: vec![],
        });

        let gateway = ContextMemoryGateway::new(vec![semantic, structural, temporal]);
        gateway
            .ingest(
                MemoryRecord {
                    namespace: "interaction-1".to_string(),
                    content: "user likes rust".to_string(),
                },
                MemoryRecord {
                    namespace: "interaction-1".to_string(),
                    content: "chat transcript".to_string(),
                },
            )
            .await
            .expect("ingest should succeed");

        assert_eq!(
            structural_writes
                .lock()
                .expect("lock should not be poisoned")
                .len(),
            1
        );
        assert_eq!(
            temporal_writes
                .lock()
                .expect("lock should not be poisoned")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn gateway_assembly_combines_results_from_registered_engines() {
        let semantic = Arc::new(InMemoryEngine {
            memory_type: MemoryProviderKind::Semantic,
            writes: Arc::new(Mutex::new(Vec::new())),
            query_results: vec![ContextSnippet {
                source: "semantic_engine".to_string(),
                memory_type: recall_proxy_core::gateway_types::MemoryType::Semantic,
                content: "semantic hit".to_string(),
                score: None,
            }],
        });
        let structural = Arc::new(InMemoryEngine {
            memory_type: MemoryProviderKind::Structural,
            writes: Arc::new(Mutex::new(Vec::new())),
            query_results: vec![ContextSnippet {
                source: "structural_engine".to_string(),
                memory_type: recall_proxy_core::gateway_types::MemoryType::Structural,
                content: "structural hit".to_string(),
                score: None,
            }],
        });
        let temporal = Arc::new(InMemoryEngine {
            memory_type: MemoryProviderKind::Temporal,
            writes: Arc::new(Mutex::new(Vec::new())),
            query_results: vec![ContextSnippet {
                source: "temporal_engine".to_string(),
                memory_type: recall_proxy_core::gateway_types::MemoryType::Temporal,
                content: "temporal hit".to_string(),
                score: None,
            }],
        });

        let gateway = ContextMemoryGateway::new(vec![semantic, structural, temporal]);
        let results = gateway
            .assemble_context(MemoryQuery {
                session_id: "s1".to_string(),
                prompt: "what do we know?".to_string(),
                max_results: 5,
            })
            .await
            .expect("assembly should succeed");

        assert_eq!(results.len(), 3);
    }
}
