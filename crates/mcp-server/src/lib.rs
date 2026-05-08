//! MCP Server — HTTP gateway for the RecallProxy MVP path.
//!
//! Exposes `/ingest` and `/context` endpoints backed by an in-memory
//! `ContextMemoryGateway`. This is the entry point for the happy-path
//! integration test.

pub mod engines;
pub mod handlers;
pub mod state;

use axum::{routing::get, routing::post, Router};
use state::McpServerState;

/// Build the MCP server router with ingest and context assembly routes.
pub fn build_router(state: McpServerState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ingest", post(handlers::ingest))
        .route("/context", post(handlers::context))
        .with_state(state)
}
