pub use crate::gateway_types::EngineError;

use async_trait::async_trait;

use crate::gateway_types::{ContextSnippet, MemoryQuery};
use crate::memory::{MemoryProviderKind, MemoryRecord};

#[async_trait]
pub trait ContextEngine: Send + Sync {
    fn memory_type(&self) -> MemoryProviderKind;

    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError>;

    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError>;
}
