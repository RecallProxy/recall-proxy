use std::sync::Arc;
use std::time::SystemTime;

use crate::domain::{ContextSnippet, EngineError, IngestReceipt, MemoryPayload, MemoryQuery};
use crate::engines::{HindsightProcessor, SemanticEngine, StructuralEngine, TemporalEngine};

pub struct ContextGateway {
    structural_engine: Arc<dyn StructuralEngine>,
    temporal_engine: Arc<dyn TemporalEngine>,
    semantic_engine: Option<Arc<dyn SemanticEngine>>,
    hindsight_processor: Option<Arc<dyn HindsightProcessor>>,
}

impl ContextGateway {
    pub fn new(
        structural_engine: Arc<dyn StructuralEngine>,
        temporal_engine: Arc<dyn TemporalEngine>,
        semantic_engine: Option<Arc<dyn SemanticEngine>>,
        hindsight_processor: Option<Arc<dyn HindsightProcessor>>,
    ) -> Self {
        Self {
            structural_engine,
            temporal_engine,
            semantic_engine,
            hindsight_processor,
        }
    }

    pub async fn ingest(&self, payload: MemoryPayload) -> Result<IngestReceipt, EngineError> {
        tokio::try_join!(
            self.structural_engine.ingest_relationships(&payload),
            self.temporal_engine.ingest_event(&payload),
            self.ingest_semantic_if_configured(&payload),
        )?;

        let scheduled_hindsight = self.schedule_hindsight(payload);

        Ok(IngestReceipt {
            ingested_at: SystemTime::now(),
            scheduled_hindsight,
        })
    }

    pub async fn assemble_context(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<ContextSnippet>, EngineError> {
        let (mut structural, mut temporal, mut semantic) = tokio::try_join!(
            self.structural_engine.query_structure(query),
            self.temporal_engine.query_timeline(query),
            self.query_semantic_if_configured(query),
        )?;

        let mut context = Vec::with_capacity(
            structural.len().saturating_add(temporal.len()) + semantic.len(),
        );
        context.append(&mut structural);
        context.append(&mut temporal);
        context.append(&mut semantic);
        Ok(context)
    }

    async fn ingest_semantic_if_configured(
        &self,
        payload: &MemoryPayload,
    ) -> Result<(), EngineError> {
        match &self.semantic_engine {
            Some(engine) => engine.upsert_embedding(payload).await,
            None => Ok(()),
        }
    }

    async fn query_semantic_if_configured(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<ContextSnippet>, EngineError> {
        match &self.semantic_engine {
            Some(engine) => engine.search_semantic(query).await,
            None => Ok(Vec::new()),
        }
    }

    fn schedule_hindsight(&self, payload: MemoryPayload) -> bool {
        if let Some(processor) = self.hindsight_processor.clone() {
            tokio::spawn(async move {
                let _ = processor.enqueue(payload).await;
            });
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::domain::{ContextSnippet, EngineError, MemoryPayload, MemoryQuery, ContextEngineType};
    use crate::engines::{HindsightProcessor, SemanticEngine, StructuralEngine, TemporalEngine};
    use crate::gateway::ContextGateway;

    #[derive(Default)]
    struct MockStructuralEngine {
        ingests: Arc<AtomicUsize>,
        snippets: Vec<ContextSnippet>,
    }

    #[async_trait]
    impl StructuralEngine for MockStructuralEngine {
        async fn ingest_relationships(&self, _payload: &MemoryPayload) -> Result<(), EngineError> {
            self.ingests.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn query_structure(
            &self,
            _query: &MemoryQuery,
        ) -> Result<Vec<ContextSnippet>, EngineError> {
            Ok(self.snippets.clone())
        }
    }

    #[derive(Default)]
    struct MockTemporalEngine {
        ingests: Arc<AtomicUsize>,
        snippets: Vec<ContextSnippet>,
    }

    #[async_trait]
    impl TemporalEngine for MockTemporalEngine {
        async fn ingest_event(&self, _payload: &MemoryPayload) -> Result<(), EngineError> {
            self.ingests.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn query_timeline(
            &self,
            _query: &MemoryQuery,
        ) -> Result<Vec<ContextSnippet>, EngineError> {
            Ok(self.snippets.clone())
        }
    }

    struct MockSemanticEngine {
        ingests: Arc<AtomicUsize>,
        snippets: Vec<ContextSnippet>,
    }

    #[async_trait]
    impl SemanticEngine for MockSemanticEngine {
        async fn upsert_embedding(&self, _payload: &MemoryPayload) -> Result<(), EngineError> {
            self.ingests.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn search_semantic(
            &self,
            _query: &MemoryQuery,
        ) -> Result<Vec<ContextSnippet>, EngineError> {
            Ok(self.snippets.clone())
        }
    }

    #[derive(Default)]
    struct MockHindsightProcessor {
        payloads: Arc<Mutex<Vec<MemoryPayload>>>,
    }

    #[async_trait]
    impl HindsightProcessor for MockHindsightProcessor {
        async fn enqueue(&self, payload: MemoryPayload) -> Result<(), EngineError> {
            self.payloads.lock().expect("lock poisoned").push(payload);
            Ok(())
        }
    }

    #[tokio::test]
    async fn ingest_routes_payload_to_primary_engines_and_schedules_hindsight() {
        let structural = MockStructuralEngine::default();
        let temporal = MockTemporalEngine::default();
        let hindsight = MockHindsightProcessor::default();
        let hindsight_payloads = hindsight.payloads.clone();

        let gateway = ContextGateway::new(
            Arc::new(structural),
            Arc::new(temporal),
            None,
            Some(Arc::new(hindsight)),
        );

        let payload = MemoryPayload {
            session_id: "s1".to_string(),
            content: "hello world".to_string(),
            metadata: HashMap::new(),
        };

        let receipt = gateway.ingest(payload).await.expect("ingest should work");
        assert!(receipt.scheduled_hindsight);

        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        assert_eq!(hindsight_payloads.lock().expect("lock poisoned").len(), 1);
    }

    #[tokio::test]
    async fn assemble_context_merges_all_configured_memory_sources() {
        let structural = MockStructuralEngine {
            ingests: Arc::new(AtomicUsize::new(0)),
            snippets: vec![ContextSnippet {
                source: "graph".to_string(),
                engine_type: ContextEngineType::Structural,
                content: "user likes rust".to_string(),
                score: None,
            }],
        };
        let temporal = MockTemporalEngine {
            ingests: Arc::new(AtomicUsize::new(0)),
            snippets: vec![ContextSnippet {
                source: "timeline".to_string(),
                engine_type: ContextEngineType::Temporal,
                content: "last action was login".to_string(),
                score: None,
            }],
        };
        let semantic = MockSemanticEngine {
            ingests: Arc::new(AtomicUsize::new(0)),
            snippets: vec![ContextSnippet {
                source: "vector".to_string(),
                engine_type: ContextEngineType::Semantic,
                content: "related prior solution".to_string(),
                score: Some(0.95),
            }],
        };

        let gateway = ContextGateway::new(
            Arc::new(structural),
            Arc::new(temporal),
            Some(Arc::new(semantic)),
            None,
        );

        let query = MemoryQuery {
            session_id: "s1".to_string(),
            prompt: "how should I solve this?".to_string(),
            max_results: 5,
        };

        let snippets = gateway
            .assemble_context(&query)
            .await
            .expect("assemble should work");
        assert_eq!(snippets.len(), 3);
    }
}
