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

    /// Create a default state with semantic, structural, temporal, and episodic engines.
    pub fn default_state() -> Self {
        let engines: Vec<Arc<InMemoryEngine>> = vec![
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Episodic)),
        ];
        Self::new(engines)
    }

    /// Build state from a `RecallProxyConfig` using the memory crate factory.
    ///
    /// This validates that all provider routes have registered providers
    /// and returns an error if startup validation fails.
    pub async fn from_config(
        config: &recall_proxy_config::RecallProxyConfig,
    ) -> Result<Self, String> {
        // Validate provider routes first
        let validation = recall_proxy_memory::factory::validate_provider_routes(config);
        if let Err(e) = validation {
            return Err(e.to_string());
        }

        // Create engines from config
        let providers = recall_proxy_memory::factory::create_all_providers(config)
            .await
            .map_err(|e| format!("failed to create providers: {e}"))?;

        // Convert providers to InMemoryEngine instances for the state
        let mut engines = Vec::new();
        for provider in providers {
            // For the MVP, all in-memory providers are wrapped as InMemoryEngine
            // since the factory creates trait objects
            let memory_type = provider.memory_type();
            let in_mem = InMemoryEngine::new(memory_type);
            engines.push(Arc::new(in_mem));
        }

        if engines.is_empty() {
            return Err("no enabled providers configured".to_string());
        }

        Ok(Self::new(engines))
    }
}
