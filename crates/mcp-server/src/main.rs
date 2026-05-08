//! RecallProxy MCP Server — memory ingest/query over MCP protocol.
//!
//! # Running locally
//!
//! ```bash
//! cargo run -p recall-proxy-mcp-server
//! ```
//!
//! The server reads JSON-RPC requests from stdin and writes responses to stdout
//! using the MCP (Model Context Protocol) transport.
//!
//! # Connecting an MCP client
//!
//! Any MCP-compatible client can connect by launching the server as a subprocess
//! with stdio transport. For example, using the MCP SDK:
//!
//! ```python
//! from mcp import ClientSession
//!
//! async with ClientSession() as session:
//!     await session.initialize()
//!     result = await session.call_tool("memory/query", {"session_id": "s1", "prompt": "hello"})
//! ```

mod engine;
mod jsonrpc;
mod server;

use engine::InMemoryEngine;
use recall_proxy_core::memory::MemoryProviderKind;
use recall_proxy_gateway::ContextMemoryGateway;
use server::McpServer;
use std::sync::Arc;

fn build_gateway() -> ContextMemoryGateway {
    let engines: Vec<Arc<dyn recall_proxy_core::engine::ContextEngine>> = vec![
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
        Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
    ];
    ContextMemoryGateway::new(engines)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let gateway = build_gateway();
    let mut server = McpServer::new(gateway);

    tracing::info!("recall-proxy-mcp-server started — waiting for JSON-RPC requests on stdin");

    loop {
        let request = match jsonrpc::read_request() {
            Some(req) => req,
            None => {
                tracing::info!("stdin closed, shutting down");
                break;
            }
        };

        let response = server.handle_request(request).await;
        jsonrpc::write_response(&response);
    }
}
