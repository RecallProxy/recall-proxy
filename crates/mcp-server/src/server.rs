//! MCP server that wires memory ingest/query through RecallProxy gateway.

use recall_proxy_core::context::ContextEngineType;
use recall_proxy_core::engine::{ContextEngine, EngineError};
use recall_proxy_core::gateway_types::{ContextSnippet, MemoryQuery};
use recall_proxy_core::memory::{MemoryProviderKind, MemoryRecord};
use recall_proxy_gateway::ContextMemoryGateway;
use std::sync::Arc;

use crate::jsonrpc::{JsonRpcRequest, JsonRpcResponse};

pub struct McpServer {
    gateway: Arc<ContextMemoryGateway>,
    initialized: bool,
}

impl McpServer {
    pub fn new(gateway: ContextMemoryGateway) -> Self {
        Self {
            gateway: Arc::new(gateway),
            initialized: false,
        }
    }

    pub async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id),
            "initialized" => JsonRpcResponse::success(id, serde_json::json!({})),
            "memory/ingest" => self.handle_memory_ingest(id, request.params).await,
            "memory/query" => self.handle_memory_query(id, request.params).await,
            _ => JsonRpcResponse::error(
                id,
                -32601,
                format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_initialize(&mut self, id: Option<serde_json::Value>) -> JsonRpcResponse {
        self.initialized = true;
        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocol_version": "2025-03-26",
                "server_info": {
                    "name": "recall-proxy-mcp-server",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "memory": {
                        "ingest": true,
                        "query": true
                    }
                }
            }),
        )
    }

    async fn handle_memory_ingest(
        &mut self,
        id: Option<serde_json::Value>,
        params: Option<serde_json::Value>,
    ) -> JsonRpcResponse {
        if !self.initialized {
            return JsonRpcResponse::error(id, -32600, "server not initialized");
        }

        let params = match params {
            Some(p) => p,
            None => return JsonRpcResponse::error(id, -32602, "missing params"),
        };

        let namespace: String = match params.get("namespace").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return JsonRpcResponse::error(id, -32602, "missing namespace"),
        };

        let content: String = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return JsonRpcResponse::error(id, -32602, "missing content"),
        };

        let record = MemoryRecord {
            namespace: namespace.clone(),
            content: content.clone(),
        };

        match self.gateway.ingest(record.clone(), record).await {
            Ok(()) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "status": "accepted",
                    "namespace": namespace,
                    "content_length": content.len()
                }),
            ),
            Err(e) => JsonRpcResponse::error(id, -32000, format!("ingest failed: {e}")),
        }
    }

    async fn handle_memory_query(
        &self,
        id: Option<serde_json::Value>,
        params: Option<serde_json::Value>,
    ) -> JsonRpcResponse {
        if !self.initialized {
            return JsonRpcResponse::error(id, -32600, "server not initialized");
        }

        let params = match params {
            Some(p) => p,
            None => return JsonRpcResponse::error(id, -32602, "missing params"),
        };

        let session_id: String = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, -32602, "missing session_id"),
        };

        let prompt: String = match params.get("prompt").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return JsonRpcResponse::error(id, -32602, "missing prompt"),
        };

        let max_results: usize = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let query = MemoryQuery {
            session_id: session_id.clone(),
            prompt: prompt.clone(),
            max_results,
        };

        match self.gateway.assemble_context(query).await {
            Ok(snippets) => {
                let results: Vec<serde_json::Value> = snippets
                    .into_iter()
                    .map(|s| {
                        let engine_type_str = match s.engine_type {
                            ContextEngineType::Structural => "structural",
                            ContextEngineType::Temporal => "temporal",
                            ContextEngineType::Semantic => "semantic",
                            ContextEngineType::Graph => "graph",
                        };
                        serde_json::json!({
                            "source": s.source,
                            "engine_type": engine_type_str,
                            "content": s.content,
                            "score": s.score,
                        })
                    })
                    .collect();
                JsonRpcResponse::success(
                    id,
                    serde_json::json!({
                        "session_id": session_id,
                        "results": results,
                        "total": results.len()
                    }),
                )
            }
            Err(e) => JsonRpcResponse::error(id, -32000, format!("query failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use recall_proxy_core::memory::MemoryProviderKind;
    use recall_proxy_gateway::ContextMemoryGateway;
    use std::sync::Arc;

    use super::*;
    use crate::engine::InMemoryEngine;

    fn make_gateway() -> ContextMemoryGateway {
        let engines: Vec<Arc<dyn ContextEngine>> = vec![
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Semantic)),
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Structural)),
            Arc::new(InMemoryEngine::new(MemoryProviderKind::Temporal)),
        ];
        ContextMemoryGateway::new(engines)
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let mut server = McpServer::new(make_gateway());
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let resp = server.handle_request(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["protocol_version"], "2025-03-26");
        assert_eq!(result["server_info"]["name"], "recall-proxy-mcp-server");
    }

    #[tokio::test]
    async fn memory_ingest_routes_through_gateway() {
        let mut server = McpServer::new(make_gateway());

        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        server.handle_request(init_req).await;

        let ingest_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "memory/ingest".to_string(),
            params: Some(serde_json::json!({
                "namespace": "test-session",
                "content": "user preference: dark mode"
            })),
            id: Some(serde_json::json!(2)),
        };
        let resp = server.handle_request(ingest_req).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["status"], "accepted");
    }

    #[tokio::test]
    async fn memory_query_routes_through_gateway() {
        let mut server = McpServer::new(make_gateway());

        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        server.handle_request(init_req).await;

        let query_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "memory/query".to_string(),
            params: Some(serde_json::json!({
                "session_id": "test-session",
                "prompt": "what do we know?"
            })),
            id: Some(serde_json::json!(2)),
        };
        let resp = server.handle_request(query_req).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert!(result["total"].is_number());
    }

    #[tokio::test]
    async fn memory_ingest_fails_before_initialize() {
        let mut server = McpServer::new(make_gateway());

        let ingest_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "memory/ingest".to_string(),
            params: Some(serde_json::json!({
                "namespace": "test",
                "content": "data"
            })),
            id: Some(serde_json::json!(1)),
        };
        let resp = server.handle_request(ingest_req).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32600);
    }

    #[tokio::test]
    async fn unknown_method_returns_error() {
        let mut server = McpServer::new(make_gateway());

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown/method".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let resp = server.handle_request(req).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }
}
