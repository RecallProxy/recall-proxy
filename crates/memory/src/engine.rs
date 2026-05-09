//! SQLite-backed memory engine implementing the `ContextEngine` trait.

use async_trait::async_trait;
use chrono::Utc;
use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::context::ContextEngineType;
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};
use sqlx::Row;
use sqlx::SqlitePool;
use tracing::info;

/// Configuration for creating a SQLite memory engine.
#[derive(Debug, Clone)]
pub struct SqliteProviderConfig {
    pub db_path: String,
}

/// A SQLite-backed memory engine that implements the `ContextEngine` trait.
pub struct SqliteMemoryEngine {
    pool: SqlitePool,
    memory_type: MemoryProviderKind,
}

impl SqliteProviderConfig {
    /// Creates a new `SqliteMemoryEngine` from this configuration.
    pub async fn build(self) -> Result<SqliteMemoryEngine, EngineError> {
        let pool = SqlitePool::connect(&format!("sqlite:{}", self.db_path))
            .await
            .map_err(|e| EngineError::new(format!("failed to connect to SQLite: {e}")))?;

        // Ensure the schema exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memory_records (
                id TEXT PRIMARY KEY,
                namespace TEXT NOT NULL,
                content TEXT NOT NULL,
                memory_type TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| EngineError::new(format!("failed to create schema: {e}")))?;

        info!(
            db_path = self.db_path,
            "sqlite memory engine initialized"
        );

        Ok(SqliteMemoryEngine {
            pool,
            memory_type: MemoryProviderKind::Semantic,
        })
    }
}

impl SqliteMemoryEngine {
    /// Creates a new `SqliteMemoryEngine` with a given pool and memory type.
    pub fn with_pool(pool: SqlitePool, memory_type: MemoryProviderKind) -> Self {
        Self { pool, memory_type }
    }

    /// Returns the memory type this engine handles.
    pub fn memory_type(&self) -> MemoryProviderKind {
        self.memory_type.clone()
    }

    /// Returns a clone of the underlying pool for testing.
    pub fn pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    fn memory_type_str(&self) -> &'static str {
        match self.memory_type {
            MemoryProviderKind::Semantic => "semantic",
            MemoryProviderKind::Structural => "structural",
            MemoryProviderKind::Temporal => "temporal",
            MemoryProviderKind::Episodic => "episodic",
        }
    }
}

#[async_trait]
impl ContextEngine for SqliteMemoryEngine {
    fn memory_type(&self) -> MemoryProviderKind {
        self.memory_type.clone()
    }

    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO memory_records (id, namespace, content, memory_type, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&record.namespace)
        .bind(&record.content)
        .bind(self.memory_type_str())
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| EngineError::new(format!("failed to write record: {e}")))?;

        info!(record_id = %id, namespace = %record.namespace, "ingested memory record");
        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError> {
        let rows = sqlx::query(
            r#"
            SELECT namespace, content, memory_type, created_at
            FROM memory_records
            WHERE namespace = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(&query.session_id)
        .bind(query.max_results as i64)
        .try_map(|row: sqlx::sqlite::SqliteRow| {
            Ok((
                row.try_get::<String, _>("namespace").unwrap_or_default(),
                row.try_get::<String, _>("content").unwrap_or_default(),
                row.try_get::<String, _>("memory_type").unwrap_or_default(),
                row.try_get::<String, _>("created_at").unwrap_or_default(),
            ))
        })
        .fetch_all(&self.pool)
        .await
        .map_err(|e| EngineError::new(format!("failed to query records: {e}")))?;

        let snippets: Vec<ContextSnippet> = rows
            .into_iter()
            .map(|(namespace, content, memory_type_str, _created_at)| {
                let engine_type = match memory_type_str.as_str() {
                    "Structural" => ContextEngineType::Structural,
                    "Temporal" => ContextEngineType::Temporal,
                    "Semantic" => ContextEngineType::Semantic,
                    _ => ContextEngineType::Semantic,
                };

                ContextSnippet {
                    source: namespace.clone(),
                    engine_type,
                    content,
                    score: Some(1.0),
                }
            })
            .collect();

        info!(
            session_id = %query.session_id,
            results = snippets.len(),
            "queried memory records"
        );

        Ok(snippets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use recall_proxy_core::context::ContextEngineType;

    async fn temp_pool() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("should connect to in-memory SQLite");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memory_records (
                id TEXT PRIMARY KEY,
                namespace TEXT NOT NULL,
                content TEXT NOT NULL,
                memory_type TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("should create schema");
        pool
    }

    #[tokio::test]
    async fn engine_writes_and_retrieves_record() {
        let pool = temp_pool().await;
        let engine = SqliteMemoryEngine::with_pool(pool, MemoryProviderKind::Semantic);

        let record = MemoryRecord {
            namespace: "test-session".to_string(),
            content: "user prefers rust".to_string(),
        };

        engine.write(record).await.expect("write should succeed");

        let results = engine
            .query(MemoryQuery {
                session_id: "test-session".to_string(),
                prompt: "what do we know?".to_string(),
                max_results: 10,
            })
            .await
            .expect("query should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "user prefers rust");
        assert_eq!(results[0].source, "test-session");
        assert_eq!(results[0].engine_type, ContextEngineType::Semantic);
        assert_eq!(results[0].score, Some(1.0));
    }

    #[tokio::test]
    async fn engine_returns_empty_for_unknown_session() {
        let pool = temp_pool().await;
        let engine = SqliteMemoryEngine::with_pool(pool, MemoryProviderKind::Semantic);

        let results = engine
            .query(MemoryQuery {
                session_id: "nonexistent".to_string(),
                prompt: "query".to_string(),
                max_results: 10,
            })
            .await
            .expect("query should succeed");

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn engine_respects_max_results_limit() {
        let pool = temp_pool().await;
        let engine = SqliteMemoryEngine::with_pool(pool, MemoryProviderKind::Semantic);

        for i in 0..5 {
            engine
                .write(MemoryRecord {
                    namespace: "limit-test".to_string(),
                    content: format!("entry-{}", i),
                })
                .await
                .expect("write should succeed");
        }

        let results = engine
            .query(MemoryQuery {
                session_id: "limit-test".to_string(),
                prompt: "query".to_string(),
                max_results: 2,
            })
            .await
            .expect("query should succeed");

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn engine_memory_type_returns_correct_kind() {
        let pool = temp_pool().await;
        let engine = SqliteMemoryEngine::with_pool(pool, MemoryProviderKind::Semantic);
        assert_eq!(engine.memory_type(), MemoryProviderKind::Semantic);

        let semantic_engine =
            SqliteMemoryEngine::with_pool(engine.pool(), MemoryProviderKind::Semantic);
        assert_eq!(semantic_engine.memory_type(), MemoryProviderKind::Semantic);
    }
}
