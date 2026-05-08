//! HTTP handlers for the MCP server endpoints.

use axum::extract::State;
use axum::Json;
use recall_proxy_core::engine::ContextEngine;
use recall_proxy_core::memory::MemoryProviderKind;
use serde::{Deserialize, Serialize};

use crate::state::McpServerState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub namespace: String,
    pub content: String,
    #[serde(default)]
    pub engine: String,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub status: String,
    pub engine: String,
}

#[derive(Debug, Deserialize)]
pub struct ContextRequest {
    pub session_id: String,
    pub prompt: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_max_results() -> usize {
    10
}

#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub snippets: Vec<ContextSnippet>,
}

#[derive(Debug, Serialize)]
pub struct ContextSnippet {
    pub source: String,
    pub memory_type: String,
    pub content: String,
    pub score: Option<f32>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Ingest a single memory record into the configured engine.
///
/// Accepts `namespace`, `content`, and `engine` (structural / temporal / semantic).
pub async fn ingest(
    State(state): State<McpServerState>,
    Json(payload): Json<IngestRequest>,
) -> Json<IngestResponse> {
    let kind = match payload.engine.as_str() {
        "structural" => MemoryProviderKind::Structural,
        "temporal" => MemoryProviderKind::Temporal,
        "semantic" => MemoryProviderKind::Semantic,
        _ => MemoryProviderKind::Structural,
    };

    let engine = state.engines.get(&kind);
    let record = recall_proxy_core::memory::MemoryRecord {
        namespace: payload.namespace.clone(),
        content: payload.content.clone(),
    };

    if let Some(engine_ref) = engine {
        let _ = engine_ref.write(record).await;
    }

    // Always write to the gateway (which routes to registered engines)
    let _ = state
        .gateway
        .ingest(
            recall_proxy_core::memory::MemoryRecord {
                namespace: format!("{}-structural", payload.namespace),
                content: payload.content.clone(),
            },
            recall_proxy_core::memory::MemoryRecord {
                namespace: format!("{}-temporal", payload.namespace),
                content: payload.content,
            },
        )
        .await;

    Json(IngestResponse {
        status: "ok".to_string(),
        engine: format!("{:?}", kind),
    })
}

/// Assemble context from all registered engines.
pub async fn context(
    State(state): State<McpServerState>,
    Json(payload): Json<ContextRequest>,
) -> Json<Result<ContextResponse, String>> {
    let query = recall_proxy_core::gateway_types::MemoryQuery {
        session_id: payload.session_id,
        prompt: payload.prompt,
        max_results: payload.max_results,
    };

    match state.gateway.assemble_context(query).await {
        Ok(snippets) => {
            let response = ContextResponse {
                snippets: snippets
                    .into_iter()
                    .map(|s| ContextSnippet {
                        source: s.source,
                        memory_type: format!("{:?}", s.memory_type),
                        content: s.content,
                        score: s.score,
                    })
                    .collect(),
            };
            Json(Ok(response))
        }
        Err(e) => Json(Err(e.to_string())),
    }
}
