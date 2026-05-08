//! Shared server state holding the gateway and in-memory engines.

use std::collections::HashMap;
use std::sync::Arc;

use recall_proxy_core::engine::ContextEngine;
use recall_proxy_core::memory::MemoryProviderKind;

use crate::engines::in_memory_engine::InMemoryEngine;

/// Shared state for the MCP server that holds all engine references.
#[derive(Clone)]
pub struct McpServerState {
    pub gateway: std::sync::Arc<recall_proxy_gateway::ContextMemoryGateway>,
    pub engines: HashMap<MemoryProviderKind, Arc<InMemoryEngine>>,
}

impl McpServerState {
    pub fn new(engines: Vec<Arc<InMemoryEngine>>) -> Self {
        let gateway = std::sync::Arc::new(recall_proxy_gateway::ContextMemoryGateway::new(
            engines
                .iter()
                .map(|e| Arc::clone(e) as Arc<dyn ContextEngine>)
                .collect(),
        ));
        let engines: HashMap<MemoryProviderKind, Arc<InMemoryEngine>> = engines
            .into_iter()
            .map(|e| (e.memory_type(), e))
            .collect();
        Self { gateway, engines }
    }

    /// Create a default state with semantic, structural, and temporal engines.
    pub fn default_state() -> Self {
        let engines: Vec<Arc<InMemoryEngine>> = vec![
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
        ];
        Self::new(engines)
    }
}
