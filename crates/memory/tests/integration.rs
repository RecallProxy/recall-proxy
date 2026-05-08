//! Integration tests for the SQLite memory provider.
//!
//! These tests verify the full ingest → query flow against SQLite
//! (in-memory), proving the provider works end-to-end.

use std::sync::Arc;

use recall_proxy_core::engine::ContextEngine;
use recall_proxy_core::gateway_types::{MemoryQuery, MemoryType};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};
use recall_proxy_memory::SqliteMemoryEngine;
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
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

async fn setup_engine() -> SqliteMemoryEngine {
    let pool = setup_pool().await;
    SqliteMemoryEngine::with_pool(pool, MemoryProviderKind::Semantic)
}

#[tokio::test]
async fn full_ingest_and_query_flow() {
    let engine = setup_engine().await;

    // Ingest a record
    let record = MemoryRecord {
        namespace: "integration-test".to_string(),
        content: "user prefers Rust for systems programming".to_string(),
    };

    engine.write(record).await.expect("write should succeed");

    // Query the record back
    let results = engine
        .query(MemoryQuery {
            session_id: "integration-test".to_string(),
            prompt: "what does the user prefer?".to_string(),
            max_results: 10,
        })
        .await
        .expect("query should succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].content,
        "user prefers Rust for systems programming"
    );
    assert_eq!(results[0].source, "integration-test");
    assert_eq!(results[0].memory_type, MemoryType::Semantic);
    assert_eq!(results[0].score, Some(1.0));
}

#[tokio::test]
async fn gateway_can_use_sqlite_engine() {
    let engine = setup_engine().await;
    let engine_arc: Arc<dyn ContextEngine> = Arc::new(engine);

    // Ingest via the trait object
    let record = MemoryRecord {
        namespace: "gateway-test".to_string(),
        content: "gateway integration test".to_string(),
    };

    engine_arc
        .write(record)
        .await
        .expect("write via trait object should succeed");

    // Query via the trait object
    let results = engine_arc
        .query(MemoryQuery {
            session_id: "gateway-test".to_string(),
            prompt: "test query".to_string(),
            max_results: 5,
        })
        .await
        .expect("query via trait object should succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "gateway integration test");
}

#[tokio::test]
async fn multiple_records_are_returned_in_order() {
    let engine = setup_engine().await;

    for i in 0..3 {
        engine
            .write(MemoryRecord {
                namespace: "ordered-test".to_string(),
                content: format!("record-{}", i),
            })
            .await
            .expect("write should succeed");
    }

    // Default ordering is DESC by created_at, so we get newest first
    let results = engine
        .query(MemoryQuery {
            session_id: "ordered-test".to_string(),
            prompt: "test".to_string(),
            max_results: 10,
        })
        .await
        .expect("query should succeed");

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].content, "record-2");
    assert_eq!(results[1].content, "record-1");
    assert_eq!(results[2].content, "record-0");
}

#[tokio::test]
async fn query_with_no_results_returns_empty() {
    let engine = setup_engine().await;

    let results = engine
        .query(MemoryQuery {
            session_id: "empty-test".to_string(),
            prompt: "test".to_string(),
            max_results: 10,
        })
        .await
        .expect("query should succeed");

    assert!(results.is_empty());
}
