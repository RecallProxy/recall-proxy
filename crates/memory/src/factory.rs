//! Provider factory for creating memory engines from configuration.

use std::sync::Arc;

use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::memory::MemoryProviderKind;
use recall_proxy_config::ProviderRegistration;

use crate::engine::SqliteMemoryEngine;

/// Creates a `ContextEngine` from a `ProviderRegistration`.
///
/// For the MVP, only the `"sqlite"` provider type is supported.
/// The `db_path` setting is read from the provider's settings map.
pub async fn create_provider(
    registration: &ProviderRegistration,
    memory_type: MemoryProviderKind,
) -> Result<Arc<dyn ContextEngine>, EngineError> {
    match registration.provider_type.as_str() {
        "sqlite" => {
            let db_path = registration
                .settings
                .get("db_path")
                .ok_or_else(|| EngineError::new("sqlite provider requires 'db_path' setting"))?;

            let pool = sqlx::SqlitePool::connect(&format!("sqlite:{}", db_path))
                .await
                .map_err(|e| EngineError::new(format!("failed to connect to SQLite: {e}")))?;

            let engine = SqliteMemoryEngine::with_pool(pool, memory_type);
            Ok(Arc::new(engine))
        }
        _ => Err(EngineError::new(format!(
            "unsupported provider type: {}",
            registration.provider_type
        ))),
    }
}

/// Creates all provider engines from a `RecallProxyConfig`.
pub async fn create_all_providers(
    config: &recall_proxy_config::RecallProxyConfig,
) -> Result<Vec<Arc<dyn ContextEngine>>, EngineError> {
    let mut engines = Vec::new();

    for registration in &config.providers {
        if !registration.enabled {
            continue;
        }

        let memory_type = match registration.provider_type.as_str() {
            "semantic" => MemoryProviderKind::Semantic,
            "structural" => MemoryProviderKind::Structural,
            "temporal" => MemoryProviderKind::Temporal,
            _ => MemoryProviderKind::Semantic,
        };

        let engine = create_provider(registration, memory_type).await?;
        engines.push(engine);
    }

    Ok(engines)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use recall_proxy_config::RecallProxyConfig;

    #[test]
    fn create_provider_rejects_unknown_type() {
        let registration = ProviderRegistration {
            id: "test".to_string(),
            provider_type: "redis".to_string(),
            enabled: true,
            capabilities: vec![],
            settings: BTreeMap::new(),
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let result = rt.block_on(create_provider(&registration, MemoryProviderKind::Semantic));
        match result {
            Ok(_) => panic!("expected error"),
            Err(ref e) => assert!(e.to_string().contains("unsupported provider type")),
        }
    }

    #[test]
    fn create_provider_requires_db_path_for_sqlite() {
        let registration = ProviderRegistration {
            id: "sqlite:memory.db".to_string(),
            provider_type: "sqlite".to_string(),
            enabled: true,
            capabilities: vec![],
            settings: BTreeMap::new(),
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let result = rt.block_on(create_provider(&registration, MemoryProviderKind::Semantic));
        match result {
            Ok(_) => panic!("expected error"),
            Err(ref e) => assert!(e.to_string().contains("db_path")),
        }
    }

    #[test]
    fn create_all_providers_skips_disabled() {
        let config = RecallProxyConfig {
            providers: vec![ProviderRegistration {
                id: "disabled".to_string(),
                provider_type: "sqlite".to_string(),
                enabled: false,
                capabilities: vec![],
                settings: BTreeMap::new(),
            }],
            read_pipelines: vec![],
            write_pipelines: vec![],
            bind_address: None,
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let engines = rt.block_on(create_all_providers(&config)).expect("should succeed");
        assert!(engines.is_empty());
    }
}
