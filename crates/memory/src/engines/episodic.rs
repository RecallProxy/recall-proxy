//! Episodic memory engine — session-bound, time-windowed storage.
//!
//! Stores memory records grouped by session with optional time-window
//! constraints for automatic pruning of stale entries.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::context::ContextEngineType;
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};

/// Configuration for the episodic engine.
#[derive(Debug, Clone)]
pub struct EpisodicEngineConfig {
    /// Maximum age of entries in minutes before automatic pruning.
    pub max_age_minutes: u64,
    /// Maximum number of entries per session.
    pub max_entries_per_session: usize,
}

impl Default for EpisodicEngineConfig {
    fn default() -> Self {
        Self {
            max_age_minutes: 1440, // 24 hours
            max_entries_per_session: 10000,
        }
    }
}

/// An entry stored in the episodic engine.
#[derive(Debug, Clone)]
struct EpisodicEntry {
    record: MemoryRecord,
    created_at: DateTime<Utc>,
    session_id: String,
}

/// In-memory episodic memory engine.
///
/// Stores records grouped by session_id with time-windowed pruning.
pub struct EpisodicEngine {
    config: EpisodicEngineConfig,
    store: Arc<RwLock<HashMap<String, Vec<EpisodicEntry>>>>,
}

impl EpisodicEngine {
    /// Creates a new episodic engine with the given configuration.
    pub fn new(config: EpisodicEngineConfig) -> Self {
        Self {
            config,
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new episodic engine with default configuration.
    pub fn with_default_config() -> Self {
        Self::new(EpisodicEngineConfig::default())
    }

    /// Prunes entries older than `max_age_minutes` from all sessions.
    fn prune(&self) {
        let now = Utc::now();
        let max_age = chrono::Duration::minutes(self.config.max_age_minutes as i64);
        let cutoff = now - max_age;

        let mut store = self.store.write().unwrap();
        for entries in store.values_mut() {
            entries.retain(|entry| entry.created_at >= cutoff);
        }
        store.retain(|_, entries| !entries.is_empty());
    }

    /// Returns the memory type this engine handles.
    pub fn memory_type(&self) -> MemoryProviderKind {
        MemoryProviderKind::Episodic
    }
}

#[async_trait]
impl ContextEngine for EpisodicEngine {
    fn memory_type(&self) -> MemoryProviderKind {
        MemoryProviderKind::Episodic
    }

    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError> {
        let session_id = if record.namespace.contains(':') {
            record.namespace.split(':').next().unwrap_or("default").to_string()
        } else {
            record.namespace.clone()
        };

        let entry = EpisodicEntry {
            record,
            created_at: Utc::now(),
            session_id: session_id.clone(),
        };

        let mut store = self.store.write().unwrap();
        let entries = store.entry(session_id).or_insert_with(Vec::new);

        if entries.len() >= self.config.max_entries_per_session {
            entries.remove(0);
        }

        entries.push(entry);
        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError> {
        self.prune();

        let store = self.store.read().unwrap();
        let mut results = Vec::new();

        for (session_id, entries) in store.iter() {
            if query.session_id.is_empty()
                || *session_id == query.session_id
                || session_id.contains(&query.session_id)
            {
                for entry in entries.iter().take(query.max_results.saturating_sub(results.len())) {
                    results.push(ContextSnippet {
                        source: session_id.clone(),
                        engine_type: ContextEngineType::Graph,
                        content: entry.record.content.clone(),
                        score: None,
                    });
                }
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
        let engine = EpisodicEngine::with_default_config();

        engine
            .write(MemoryRecord {
                namespace: "session-1".to_string(),
                content: "episodic memory 1".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "session-1".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "episodic memory 1");
    }

    #[tokio::test]
    async fn queries_by_session_id() {
        let engine = EpisodicEngine::with_default_config();

        engine
            .write(MemoryRecord {
                namespace: "session-a".to_string(),
                content: "session a data".to_string(),
            })
            .await
            .unwrap();

        engine
            .write(MemoryRecord {
                namespace: "session-b".to_string(),
                content: "session b data".to_string(),
            })
            .await
            .unwrap();

        let results = engine
            .query(MemoryQuery {
                session_id: "session-a".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "session a data");
    }

    #[tokio::test]
    async fn returns_empty_for_unknown_session() {
        let engine = EpisodicEngine::with_default_config();

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
    async fn respects_max_entries_per_session() {
        let config = EpisodicEngineConfig {
            max_age_minutes: 1440,
            max_entries_per_session: 3,
        };
        let engine = EpisodicEngine::new(config);

        for i in 0..5 {
            engine
                .write(MemoryRecord {
                    namespace: "session-1".to_string(),
                    content: format!("entry-{}", i),
                })
                .await
                .unwrap();
        }

        let results = engine
            .query(MemoryQuery {
                session_id: "session-1".to_string(),
                prompt: "".to_string(),
                max_results: 10,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn memory_type_returns_episodic() {
        let engine = EpisodicEngine::with_default_config();
        assert_eq!(engine.memory_type(), MemoryProviderKind::Episodic);
    }
}
