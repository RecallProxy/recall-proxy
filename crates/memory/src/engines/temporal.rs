//! Temporal memory engine — chronological timeline storage.
//!
//! Stores memory records with timestamps and supports time-range
//! queries for retrieving events within specific windows.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::context::ContextEngineType;
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};

/// Configuration for the temporal engine.
#[derive(Debug, Clone)]
pub struct TemporalEngineConfig {
    /// Maximum number of entries to retain.
    pub max_entries: usize,
    /// Whether to enable time-range filtering.
    pub enable_time_filtering: bool,
}

impl Default for TemporalEngineConfig {
    fn default() -> Self {
        Self {
            max_entries: 100000,
            enable_time_filtering: true,
        }
    }
}

/// A time-stamped entry in the temporal engine.
#[derive(Debug, Clone)]
struct TemporalEntry {
    record: MemoryRecord,
    timestamp: DateTime<Utc>,
    event_type: String,
}

/// In-memory temporal memory engine.
///
/// Stores records chronologically with time-range query support.
pub struct TemporalEngine {
    config: TemporalEngineConfig,
    store: Arc<RwLock<BTreeMap<DateTime<Utc>, TemporalEntry>>>,
}

impl TemporalEngine {
    /// Creates a new temporal engine with the given configuration.
    pub fn new(config: TemporalEngineConfig) -> Self {
        Self {
            config,
            store: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Creates a new temporal engine with default configuration.
    pub fn with_default_config() -> Self {
        Self::new(TemporalEngineConfig::default())
    }

    /// Prunes oldest entries if the store exceeds `max_entries`.
    fn prune(&self) {
        let mut store = self.store.write().unwrap();
        while store.len() > self.config.max_entries {
            store.pop_first();
        }
    }

    /// Returns the memory type this engine handles.
    pub fn memory_type(&self) -> MemoryProviderKind {
        MemoryProviderKind::Temporal
    }
}

#[async_trait]
impl ContextEngine for TemporalEngine {
    fn memory_type(&self) -> MemoryProviderKind {
        MemoryProviderKind::Temporal
    }

    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError> {
        let timestamp = Utc::now();
        let event_type = if record.namespace.contains("transcript") {
            "transcript".to_string()
        } else if record.namespace.contains("event") {
            "event".to_string()
        } else {
            "memory".to_string()
        };

        let entry = TemporalEntry {
            record,
            timestamp,
            event_type,
        };

        let mut store = self.store.write().unwrap();
        store.insert(timestamp, entry);
        drop(store);

        self.prune();
        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError> {
        let store = self.store.read().unwrap();
        let mut results = Vec::new();

        for entry in store.iter().rev() {
            if query.session_id.is_empty()
                || entry.1.record.namespace.contains(&query.session_id)
                || query.session_id.contains(&entry.1.record.namespace)
            {
                results.push(ContextSnippet {
                    source: entry.1.record.namespace.clone(),
                    engine_type: ContextEngineType::Temporal,
                    content: entry.1.record.content.clone(),
                    score: Some(1.0),
                });
            }

            if results.len() >= query.max_results {
                break;
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_and_retrieves_records() {
        let engine = TemporalEngine::with_default_config();

        engine
            .write(MemoryRecord {
                namespace: "timeline-1".to_string(),
                content: "temporal event 1".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "timeline-1".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "temporal event 1");
    }

    #[tokio::test]
    async fn returns_entries_in_reverse_chronological_order() {
        let engine = TemporalEngine::with_default_config();

        for i in 0..3 {
            engine
                .write(MemoryRecord {
                    namespace: "timeline".to_string(),
                    content: format!("event-{}", i),
                })
                .await
                .unwrap();
        }

        let results = engine
            .query(MemoryQuery {
                session_id: "timeline".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].content, "event-2");
        assert_eq!(results[1].content, "event-1");
        assert_eq!(results[2].content, "event-0");
    }

    #[tokio::test]
    async fn returns_empty_for_unknown_session() {
        let engine = TemporalEngine::with_default_config();

        let results = engine
            .query(MemoryQuery {
                session_id: "nonexistent".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn memory_type_returns_temporal() {
        let engine = TemporalEngine::with_default_config();
        assert_eq!(engine.memory_type(), MemoryProviderKind::Temporal);
    }
}
