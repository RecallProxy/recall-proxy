use async_trait::async_trait;

use crate::domain::{ContextSnippet, EngineError, MemoryPayload, MemoryQuery};

#[async_trait]
pub trait StructuralEngine: Send + Sync {
    async fn ingest_relationships(&self, payload: &MemoryPayload) -> Result<(), EngineError>;
    async fn query_structure(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<ContextSnippet>, EngineError>;
}

#[async_trait]
pub trait TemporalEngine: Send + Sync {
    async fn ingest_event(&self, payload: &MemoryPayload) -> Result<(), EngineError>;
    async fn query_timeline(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<ContextSnippet>, EngineError>;
}

#[async_trait]
pub trait SemanticEngine: Send + Sync {
    async fn upsert_embedding(&self, payload: &MemoryPayload) -> Result<(), EngineError>;
    async fn search_semantic(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<ContextSnippet>, EngineError>;
}

#[async_trait]
pub trait HindsightProcessor: Send + Sync {
    async fn enqueue(&self, payload: MemoryPayload) -> Result<(), EngineError>;
}
