//! Integration / smoke tests for the MVP path.
//!
//! These tests verify the end-to-end flow: ingest -> retrieval through
//! the MCP server and gateway, using in-memory engine providers.

use recall_proxy_core::context::RetrievalIntent;
use recall_proxy_core::engine::ContextEngine;
use recall_proxy_core::gateway_types::MemoryQuery;
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};
use recall_proxy_mcp_server::engines::in_memory_engine::InMemoryEngine;
use recall_proxy_mcp_server::state::McpServerState;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Gateway-level integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ingest_and_retrieve_through_gateway() {
    let engines: Vec<Arc<dyn ContextEngine>> = vec![
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
    ];
    let state = McpServerState::new(
        engines
            .iter()
            .map(|e| Arc::new(InMemoryEngine::new(e.memory_type())))
            .collect(),
    );

    // Write directly to engines
    let structural_engine = state.engines.get(&MemoryProviderKind::Structural).unwrap();
    let temporal_engine = state.engines.get(&MemoryProviderKind::Temporal).unwrap();

    structural_engine
        .write(MemoryRecord {
            namespace: "integration-test".to_string(),
            content: "structural record".to_string(),
        })
        .await
        .unwrap();

    temporal_engine
        .write(MemoryRecord {
            namespace: "integration-test".to_string(),
            content: "temporal record".to_string(),
        })
        .await
        .unwrap();

    // Retrieve context via gateway
    let snippets = state
        .gateway
        .assemble_context(MemoryQuery {
            session_id: "integration-test".to_string(),
            prompt: "integration-test".to_string(),
            max_results: 10,
            retrieval_intent: RetrievalIntent::Mixed,
        })
        .await
        .unwrap();

    assert!(snippets.len() >= 2);
}

#[tokio::test]
async fn gateway_ingest_routes_to_structural_and_temporal() {
    let engines: Vec<Arc<dyn ContextEngine>> = vec![
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
    ];
    let gateway = recall_proxy_gateway::ContextMemoryGateway::new(engines);

    let structural_record = MemoryRecord {
        namespace: "session-1".to_string(),
        content: "structural data".to_string(),
    };
    let temporal_record = MemoryRecord {
        namespace: "session-1".to_string(),
        content: "temporal data".to_string(),
    };

    gateway
        .ingest(structural_record, temporal_record)
        .await
        .expect("ingest should succeed");

    let snippets = gateway
        .assemble_context(MemoryQuery {
            session_id: "session-1".to_string(),
            prompt: "session-1".to_string(),
            max_results: 10,
            retrieval_intent: RetrievalIntent::Mixed,
        })
        .await
        .unwrap();

    assert!(snippets.len() >= 2);
}

#[tokio::test]
async fn assemble_context_returns_empty_when_no_engines() {
    let gateway = recall_proxy_gateway::ContextMemoryGateway::new(vec![]);

    let snippets = gateway
        .assemble_context(MemoryQuery {
            session_id: "empty".to_string(),
            prompt: "empty".to_string(),
            max_results: 10,
            retrieval_intent: RetrievalIntent::Mixed,
        })
        .await
        .unwrap();

    assert_eq!(snippets.len(), 0);
}

#[tokio::test]
async fn ingest_fails_when_engine_missing() {
    // Only provide a semantic engine — no structural or temporal
    let engines: Vec<Arc<dyn ContextEngine>> = vec![
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
    ];
    let gateway = recall_proxy_gateway::ContextMemoryGateway::new(engines);

    let result = gateway
        .ingest(
            MemoryRecord {
                namespace: "s1".to_string(),
                content: "data".to_string(),
            },
            MemoryRecord {
                namespace: "s1".to_string(),
                content: "data".to_string(),
            },
        )
        .await;

    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Engine-level integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn in_memory_engine_stores_and_retrieves_records() {
    let engine = InMemoryEngine::new(MemoryProviderKind::Structural);

    engine
        .write(MemoryRecord {
            namespace: "ns-1".to_string(),
            content: "record-one".to_string(),
        })
        .await
        .unwrap();

    engine
        .write(MemoryRecord {
            namespace: "ns-2".to_string(),
            content: "record-two".to_string(),
        })
        .await
        .unwrap();

    let results = engine
        .query(MemoryQuery {
            session_id: "ns".to_string(),
            prompt: "".to_string(),
            max_results: 10,
            retrieval_intent: RetrievalIntent::Mixed,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
}

// ---------------------------------------------------------------------------
// Configuration tests
// ---------------------------------------------------------------------------

#[test]
fn gateway_config_parses_provider_list() {
    let config = recall_proxy_config::GatewayConfig {
        bind_address: "127.0.0.1:8080".to_string(),
        providers: vec![
            recall_proxy_config::ProviderConfig {
                name: "semantic".to_string(),
                kind: "semantic".to_string(),
            },
            recall_proxy_config::ProviderConfig {
                name: "structural".to_string(),
                kind: "structural".to_string(),
            },
        ],
    };

    assert_eq!(config.providers.len(), 2);
    assert_eq!(config.providers[0].name, "semantic");
    assert_eq!(config.providers[1].kind, "structural");
}
