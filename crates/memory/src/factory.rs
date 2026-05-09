//! Provider factory for creating memory engines from configuration.

use std::sync::Arc;

use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::memory::MemoryProviderKind;
use recall_proxy_config::ProviderRegistration;

use crate::engine::SqliteMemoryEngine;
use crate::engines::{
    EpisodicEngine, EpisodicEngineConfig, SemanticEngine, SemanticEngineConfig,
    StructuralEngine, StructuralEngineConfig, TemporalEngine, TemporalEngineConfig,
};

/// Creates a `ContextEngine` from a `ProviderRegistration`.
///
/// Supports the following `provider_type` values:
/// - `"sqlite"` — SQLite-backed engine (requires `db_path` setting)
/// - `"episodic"` — In-memory episodic engine
/// - `"semantic"` — In-memory semantic engine
/// - `"temporal"` — In-memory temporal engine
/// - `"structural"` — In-memory structural engine
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
        "episodic" => {
            let max_age = registration
                .settings
                .get("max_age_minutes")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1440);

            let max_entries = registration
                .settings
                .get("max_entries_per_session")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(10000);

            let engine = EpisodicEngine::new(EpisodicEngineConfig {
                max_age_minutes: max_age,
                max_entries_per_session: max_entries,
            });
            Ok(Arc::new(engine))
        }
        "semantic" => {
            let max_results = registration
                .settings
                .get("max_results")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(50);

            let engine = SemanticEngine::new(SemanticEngineConfig {
                max_results,
                enable_keyword_search: true,
            });
            Ok(Arc::new(engine))
        }
        "temporal" => {
            let max_entries = registration
                .settings
                .get("max_entries")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(100000);

            let engine = TemporalEngine::new(TemporalEngineConfig {
                max_entries,
                enable_time_filtering: true,
            });
            Ok(Arc::new(engine))
        }
        "structural" => {
            let max_facts = registration
                .settings
                .get("max_facts")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(50000);

            let engine = StructuralEngine::new(StructuralEngineConfig {
                max_facts,
                enable_relationships: true,
            });
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
            "episodic" => MemoryProviderKind::Episodic,
            _ => MemoryProviderKind::Semantic,
        };

        let engine = create_provider(registration, memory_type).await?;
        engines.push(engine);
    }

    Ok(engines)
}

/// Validates that all provider routes in the config have a corresponding
/// registered provider. Returns a list of missing provider IDs.
pub fn validate_provider_routes(
    config: &recall_proxy_config::RecallProxyConfig,
) -> Result<Vec<String>, EngineError> {
    let registered_ids: std::collections::HashSet<&str> = config
        .providers
        .iter()
        .filter(|p| p.enabled)
        .map(|p| p.id.as_str())
        .collect();

    let mut missing = Vec::new();

    for pipeline in &config.read_pipelines {
        for route in &pipeline.providers {
            if !registered_ids.contains(route.provider_id.as_str()) {
                missing.push(format!(
                    "read_pipeline[{}].provider[{}]: provider '{}' not found",
                    pipeline.id, route.provider_id, route.provider_id
                ));
            }
        }
    }

    for pipeline in &config.write_pipelines {
        for sink in &pipeline.sinks {
            if !registered_ids.contains(sink.provider_id.as_str()) {
                missing.push(format!(
                    "write_pipeline[{}].sink[{}]: provider '{}' not found",
                    pipeline.id, sink.provider_id, sink.provider_id
                ));
            }
        }
    }

    if missing.is_empty() {
        Ok(missing)
    } else {
        Err(EngineError::new(format!(
            "startup validation failed: {} missing provider(s): {}",
            missing.len(),
            missing.join("; ")
        )))
    }
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

    #[tokio::test]
    async fn create_episodic_provider_succeeds() {
        let registration = ProviderRegistration {
            id: "episodic-1".to_string(),
            provider_type: "episodic".to_string(),
            enabled: true,
            capabilities: vec!["session_memory".to_string()],
            settings: BTreeMap::new(),
        };

        let engine = create_provider(&registration, MemoryProviderKind::Episodic)
            .await
            .expect("episodic provider should succeed");

        assert_eq!(engine.memory_type(), MemoryProviderKind::Episodic);
    }

    #[tokio::test]
    async fn create_semantic_provider_succeeds() {
        let registration = ProviderRegistration {
            id: "semantic-1".to_string(),
            provider_type: "semantic".to_string(),
            enabled: true,
            capabilities: vec!["semantic_search".to_string()],
            settings: BTreeMap::new(),
        };

        let engine = create_provider(&registration, MemoryProviderKind::Semantic)
            .await
            .expect("semantic provider should succeed");

        assert_eq!(engine.memory_type(), MemoryProviderKind::Semantic);
    }

    #[tokio::test]
    async fn create_temporal_provider_succeeds() {
        let registration = ProviderRegistration {
            id: "temporal-1".to_string(),
            provider_type: "temporal".to_string(),
            enabled: true,
            capabilities: vec!["timeline".to_string()],
            settings: BTreeMap::new(),
        };

        let engine = create_provider(&registration, MemoryProviderKind::Temporal)
            .await
            .expect("temporal provider should succeed");

        assert_eq!(engine.memory_type(), MemoryProviderKind::Temporal);
    }

    #[tokio::test]
    async fn create_structural_provider_succeeds() {
        let registration = ProviderRegistration {
            id: "structural-1".to_string(),
            provider_type: "structural".to_string(),
            enabled: true,
            capabilities: vec!["graph_neighborhood".to_string()],
            settings: BTreeMap::new(),
        };

        let engine = create_provider(&registration, MemoryProviderKind::Structural)
            .await
            .expect("structural provider should succeed");

        assert_eq!(engine.memory_type(), MemoryProviderKind::Structural);
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

    #[test]
    fn create_all_providers_includes_all_enabled() {
        let config = RecallProxyConfig {
            providers: vec![
                ProviderRegistration {
                    id: "episodic-1".to_string(),
                    provider_type: "episodic".to_string(),
                    enabled: true,
                    capabilities: vec![],
                    settings: BTreeMap::new(),
                },
                ProviderRegistration {
                    id: "semantic-1".to_string(),
                    provider_type: "semantic".to_string(),
                    enabled: true,
                    capabilities: vec![],
                    settings: BTreeMap::new(),
                },
                ProviderRegistration {
                    id: "temporal-1".to_string(),
                    provider_type: "temporal".to_string(),
                    enabled: true,
                    capabilities: vec![],
                    settings: BTreeMap::new(),
                },
                ProviderRegistration {
                    id: "structural-1".to_string(),
                    provider_type: "structural".to_string(),
                    enabled: true,
                    capabilities: vec![],
                    settings: BTreeMap::new(),
                },
            ],
            read_pipelines: vec![],
            write_pipelines: vec![],
            bind_address: None,
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let engines = rt
            .block_on(create_all_providers(&config))
            .expect("should succeed");
        assert_eq!(engines.len(), 4);
    }

    #[test]
    fn validate_provider_routes_passes_when_all_present() {
        let config = RecallProxyConfig {
            providers: vec![
                ProviderRegistration {
                    id: "semantic-1".to_string(),
                    provider_type: "semantic".to_string(),
                    enabled: true,
                    capabilities: vec![],
                    settings: BTreeMap::new(),
                },
                ProviderRegistration {
                    id: "structural-1".to_string(),
                    provider_type: "structural".to_string(),
                    enabled: true,
                    capabilities: vec![],
                    settings: BTreeMap::new(),
                },
            ],
            read_pipelines: vec![recall_proxy_config::ReadPipeline {
                id: "default".to_string(),
                providers: vec![
                    recall_proxy_config::ReadProviderRoute {
                        provider_id: "semantic-1".to_string(),
                        capability: "semantic_search".to_string(),
                        priority: 10,
                        weight: 100,
                        enabled: true,
                    },
                    recall_proxy_config::ReadProviderRoute {
                        provider_id: "structural-1".to_string(),
                        capability: "graph_neighborhood".to_string(),
                        priority: 5,
                        weight: 50,
                        enabled: true,
                    },
                ],
            }],
            write_pipelines: vec![recall_proxy_config::WritePipeline {
                id: "archive".to_string(),
                trigger: recall_proxy_config::WriteTrigger::OnResponse,
                sinks: vec![recall_proxy_config::WriteSink {
                    provider_id: "semantic-1".to_string(),
                    capability: "semantic_search".to_string(),
                    enabled: true,
                    criticality: recall_proxy_config::WriteCriticality::Required,
                }],
            }],
            bind_address: None,
        };

        let result = validate_provider_routes(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_provider_routes_fails_when_missing_provider() {
        let config = RecallProxyConfig {
            providers: vec![ProviderRegistration {
                id: "semantic-1".to_string(),
                provider_type: "semantic".to_string(),
                enabled: true,
                capabilities: vec![],
                settings: BTreeMap::new(),
            }],
            read_pipelines: vec![recall_proxy_config::ReadPipeline {
                id: "default".to_string(),
                providers: vec![recall_proxy_config::ReadProviderRoute {
                    provider_id: "missing-provider".to_string(),
                    capability: "semantic_search".to_string(),
                    priority: 10,
                    weight: 100,
                    enabled: true,
                }],
            }],
            write_pipelines: vec![],
            bind_address: None,
        };

        let result = validate_provider_routes(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("missing provider"));
        assert!(err.to_string().contains("missing-provider"));
    }
}
