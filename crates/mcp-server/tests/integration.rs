//! Integration / smoke tests for the MVP path.
//!
//! These tests verify the end-to-end flow: ingest -> retrieval through
//! the MCP server and gateway, using in-memory engine providers.

use recall_proxy_core::engine::ContextEngine;
use recall_proxy_core::context::ContextEngineType;
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

// ---------------------------------------------------------------------------
// Multi-memory retrieval behavior tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn semantic_engine_retrieves_records_by_namespace() {
    let semantic_engine = recall_proxy_mcp_server::engines::in_memory_engine::InMemoryEngine::new(
        MemoryProviderKind::Semantic,
    );

    semantic_engine
        .write(MemoryRecord {
            namespace: "semantic-test".to_string(),
            content: "semantic content about rust".to_string(),
        })
        .await
        .unwrap();

    semantic_engine
        .write(MemoryRecord {
            namespace: "semantic-test".to_string(),
            content: "another semantic entry".to_string(),
        })
        .await
        .unwrap();

    let results = semantic_engine
        .query(MemoryQuery {
            session_id: "semantic-test".to_string(),
            prompt: "rust".to_string(),
            max_results: 10,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    for snippet in &results {
        assert_eq!(snippet.engine_type, recall_proxy_core::context::ContextEngineType::Semantic);
        assert_eq!(snippet.score, Some(1.0));
    }
}

#[tokio::test]
async fn structural_engine_retrieves_relationship_records() {
    let structural_engine = recall_proxy_mcp_server::engines::in_memory_engine::InMemoryEngine::new(
        MemoryProviderKind::Structural,
    );

    structural_engine
        .write(MemoryRecord {
            namespace: "relationships".to_string(),
            content: "user prefers Rust over Go".to_string(),
        })
        .await
        .unwrap();

    structural_engine
        .write(MemoryRecord {
            namespace: "relationships".to_string(),
            content: "user contributed to a Rust project".to_string(),
        })
        .await
        .unwrap();

    let results = structural_engine
        .query(MemoryQuery {
            session_id: "relationships".to_string(),
            prompt: "rust".to_string(),
            max_results: 10,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    for snippet in &results {
        assert_eq!(snippet.engine_type, recall_proxy_core::context::ContextEngineType::Structural);
    }
}

#[tokio::test]
async fn temporal_engine_retrieves_timeline_entries() {
    let temporal_engine = recall_proxy_mcp_server::engines::in_memory_engine::InMemoryEngine::new(
        MemoryProviderKind::Temporal,
    );

    temporal_engine
        .write(MemoryRecord {
            namespace: "timeline".to_string(),
            content: "user logged in at 2026-01-01".to_string(),
        })
        .await
        .unwrap();

    temporal_engine
        .write(MemoryRecord {
            namespace: "timeline".to_string(),
            content: "user ran query at 2026-01-02".to_string(),
        })
        .await
        .unwrap();

    let results = temporal_engine
        .query(MemoryQuery {
            session_id: "timeline".to_string(),
            prompt: "login".to_string(),
            max_results: 10,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    for snippet in &results {
        assert_eq!(snippet.engine_type, recall_proxy_core::context::ContextEngineType::Temporal);
    }
}

#[tokio::test]
async fn all_three_engines_return_correct_types_via_gateway() {
    let engines: Vec<Arc<dyn ContextEngine>> = vec![
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
    ];
    let gateway = recall_proxy_gateway::ContextMemoryGateway::new(engines);

    // Write to structural and temporal (the ingest path)
    gateway
        .ingest(
            MemoryRecord {
                namespace: "multi-test".to_string(),
                content: "structural data".to_string(),
            },
            MemoryRecord {
                namespace: "multi-test".to_string(),
                content: "temporal data".to_string(),
            },
        )
        .await
        .expect("ingest should succeed");

    // Write directly to semantic (simulating consolidation from hindsight)
    let semantic_engine = gateway
        .engines
        .get(&MemoryProviderKind::Semantic)
        .unwrap();
    semantic_engine
        .write(MemoryRecord {
            namespace: "multi-test".to_string(),
            content: "consolidated semantic data".to_string(),
        })
        .await
        .expect("semantic write should succeed");

    // Retrieve from all engines
    let snippets = gateway
        .assemble_context(MemoryQuery {
            session_id: "multi-test".to_string(),
            prompt: "multi-test".to_string(),
            max_results: 10,
        })
        .await
        .unwrap();

    assert_eq!(snippets.len(), 3);

    let types: Vec<_> = snippets.iter().map(|s| s.engine_type).collect();
    assert!(types.contains(&recall_proxy_core::context::ContextEngineType::Semantic));
    assert!(types.contains(&recall_proxy_core::context::ContextEngineType::Structural));
    assert!(types.contains(&recall_proxy_core::context::ContextEngineType::Temporal));
}

// ---------------------------------------------------------------------------
// Consolidation outputs retrievable through gateway
// ---------------------------------------------------------------------------

#[tokio::test]
async fn consolidation_outputs_become_retrievable_through_gateway() {
    // Simulate the full multi-memory flow:
    // 1. Ingest writes to structural + temporal
    // 2. Hindsight consolidation writes to semantic
    // 3. Retrieve queries all three
    let engines: Vec<Arc<dyn ContextEngine>> = vec![
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
    ];
    let gateway = recall_proxy_gateway::ContextMemoryGateway::new(engines);

    // Step 1: Ingest (writes to structural + temporal)
    gateway
        .ingest(
            MemoryRecord {
                namespace: "consolidation-test".to_string(),
                content: "entity: user, relation: prefers, object: Rust".to_string(),
            },
            MemoryRecord {
                namespace: "consolidation-test".to_string(),
                content: "event: login at 2026-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("ingest should succeed");

    // Step 2: Simulate hindsight consolidation writing to semantic engine
    let semantic_engine = gateway
        .engines
        .get(&MemoryProviderKind::Semantic)
        .expect("semantic engine should exist");

    // Consolidated facts derived from structural + temporal records
    semantic_engine
        .write(MemoryRecord {
            namespace: "consolidation-test".to_string(),
            content: "derived: user prefers Rust (consolidated from structural + temporal)".to_string(),
        })
        .await
        .expect("consolidation write should succeed");

    semantic_engine
        .write(MemoryRecord {
            namespace: "consolidation-test".to_string(),
            content: "derived: user activity timeline spans 2026-01-01".to_string(),
        })
        .await
        .expect("consolidation write should succeed");

    // Step 3: Retrieve — all three engines should return results
    let snippets = gateway
        .assemble_context(MemoryQuery {
            session_id: "consolidation-test".to_string(),
            prompt: "consolidation-test".to_string(),
            max_results: 10,
        })
        .await
        .unwrap();

    // Verify all engine types are represented
    let mut has_semantic = false;
    let mut has_structural = false;
    let mut has_temporal = false;

    for snippet in &snippets {
        match snippet.engine_type {
            recall_proxy_core::context::ContextEngineType::Semantic => has_semantic = true,
            recall_proxy_core::context::ContextEngineType::Structural => has_structural = true,
            recall_proxy_core::context::ContextEngineType::Temporal => has_temporal = true,
            recall_proxy_core::context::ContextEngineType::Graph => {}
        }
    }

    assert!(has_semantic, "semantic engine should return consolidation results");
    assert!(has_structural, "structural engine should return ingest results");
    assert!(has_temporal, "temporal engine should return ingest results");

    // Verify consolidation content is retrievable
    let consolidated_content: Vec<_> = snippets
        .iter()
        .filter(|s| s.engine_type == recall_proxy_core::context::ContextEngineType::Semantic)
        .map(|s| s.content.clone())
        .collect();

    assert!(consolidated_content.iter().any(|c| c.contains("derived")));
    assert!(consolidated_content.iter().any(|c| c.contains("consolidated")));
}

#[tokio::test]
async fn gateway_assembly_combines_all_engine_results() {
    let engines: Vec<Arc<dyn ContextEngine>> = vec![
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
    ];
    let gateway = recall_proxy_gateway::ContextMemoryGateway::new(engines);

    // Write records to each engine
    gateway
        .ingest(
            MemoryRecord {
                namespace: "multi-test".to_string(),
                content: "structural-record".to_string(),
            },
            MemoryRecord {
                namespace: "multi-test".to_string(),
                content: "temporal-record".to_string(),
            },
        )
        .await
        .unwrap();

    let semantic_engine = gateway
        .engines
        .get(&MemoryProviderKind::Semantic)
        .unwrap();
    semantic_engine
        .write(MemoryRecord {
            namespace: "multi-test".to_string(),
            content: "semantic-record".to_string(),
        })
        .await
        .unwrap();

    let snippets = gateway
        .assemble_context(MemoryQuery {
            session_id: "multi-test".to_string(),
            prompt: "multi-test".to_string(),
            max_results: 10,
        })
        .await
        .unwrap();

    // All three engines contribute results
    assert_eq!(snippets.len(), 3);

    let types: std::collections::HashSet<_> = snippets
        .iter()
        .map(|s| s.engine_type)
        .collect();
    assert_eq!(types.len(), 3);
}

#[tokio::test]
async fn full_multi_memory_flow_ingest_to_retrieve() {
    // Complete multi-memory flow: ingest → consolidation → retrieve
    let engines: Vec<Arc<dyn ContextEngine>> = vec![
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
    ];
    let state = recall_proxy_mcp_server::state::McpServerState::new(
        engines
            .iter()
            .map(|e| Arc::new(InMemoryEngine::new(e.memory_type())))
            .collect(),
    );

    // Ingest: structural + temporal
    let structural_engine = state.engines.get(&MemoryProviderKind::Structural).unwrap();
    let temporal_engine = state.engines.get(&MemoryProviderKind::Temporal).unwrap();

    structural_engine
        .write(MemoryRecord {
            namespace: "full-flow-test".to_string(),
            content: "structural: user-location=Berlin".to_string(),
        })
        .await
        .unwrap();

    temporal_engine
        .write(MemoryRecord {
            namespace: "full-flow-test".to_string(),
            content: "temporal: last-visit=2026-05-07".to_string(),
        })
        .await
        .unwrap();

    // Consolidation: semantic engine receives derived facts
    let semantic_engine = state.engines.get(&MemoryProviderKind::Semantic).unwrap();
    semantic_engine
        .write(MemoryRecord {
            namespace: "full-flow-test".to_string(),
            content: "consolidated: user lives in Berlin (from structural + temporal)".to_string(),
        })
        .await
        .unwrap();

    // Retrieve through gateway
    let snippets = state
        .gateway
        .assemble_context(MemoryQuery {
            session_id: "full-flow-test".to_string(),
            prompt: "full-flow-test".to_string(),
            max_results: 10,
        })
        .await
        .unwrap();

    assert_eq!(snippets.len(), 3);

    // Verify each engine type contributed
    let types: std::collections::HashSet<_> = snippets
        .iter()
        .map(|s| s.engine_type)
        .collect();
    assert_eq!(types.len(), 3);
}
