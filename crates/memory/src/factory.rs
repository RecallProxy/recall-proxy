//! Provider factory for creating memory engines from configuration.

use std::sync::Arc;

use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::memory::MemoryProviderKind;
use recall_proxy_config::{GatewayConfig, ProviderConfig};

use crate::engine::SqliteMemoryEngine;

/// Creates a `ContextEngine` from the given gateway configuration.
///
/// For the MVP, only the `"sqlite"` provider kind is supported.
/// Future provider types can be added here as match arms.
pub async fn create_provider(
    config: &ProviderConfig,
    memory_type: MemoryProviderKind,
) -> Result<Arc<dyn ContextEngine>, EngineError> {
    match config.kind.as_str() {
        "sqlite" => {
            let db_path = config
                .name
                .strip_prefix("sqlite:")
                .ok_or_else(|| EngineError::new("sqlite provider name must start with 'sqlite:'"))?;

            let pool = sqlx::SqlitePool::connect(&format!("sqlite:{}", db_path))
                .await
                .map_err(|e| EngineError::new(format!("failed to connect to SQLite: {e}")))?;

            let engine = SqliteMemoryEngine::with_pool(pool, memory_type);
            Ok(Arc::new(engine))
        }
        _ => Err(EngineError::new(format!(
            "unsupported provider kind: {}",
            config.kind
        ))),
    }
}

/// Creates all provider engines from a full gateway configuration.
pub async fn create_all_providers(
    gateway_config: &GatewayConfig,
) -> Result<Vec<Arc<dyn ContextEngine>>, EngineError> {
    let mut engines = Vec::new();

    for provider_config in &gateway_config.providers {
        let memory_type = match provider_config.kind.as_str() {
            "semantic" => MemoryProviderKind::Semantic,
            "structural" => MemoryProviderKind::Structural,
            "temporal" => MemoryProviderKind::Temporal,
            _ => MemoryProviderKind::Semantic,
        };

        let engine = create_provider(provider_config, memory_type).await?;
        engines.push(engine);
    }

    Ok(engines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use recall_proxy_config::GatewayConfig;

    #[test]
    fn create_provider_rejects_unknown_kind() {
        let config = ProviderConfig {
            name: "test".to_string(),
            kind: "unknown".to_string(),
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let result = rt.block_on(create_provider(&config, MemoryProviderKind::Semantic));
        match result {
            Ok(_) => panic!("expected error"),
            Err(ref e) => assert!(e.to_string().contains("unsupported provider kind")),
        }
    }

    #[test]
    fn create_all_providers_returns_empty_for_no_providers() {
        let config = GatewayConfig {
            bind_address: "127.0.0.1:8080".to_string(),
            providers: vec![],
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let engines = rt.block_on(create_all_providers(&config)).expect("should succeed");
        assert!(engines.is_empty());
    }
}
