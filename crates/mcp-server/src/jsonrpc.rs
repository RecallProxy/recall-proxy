//! JSON-RPC 2.0 transport over stdio for the MCP protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
            }),
            id,
        }
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            eprintln!("failed to serialize JSON-RPC response: {e}");
            r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"internal error"},"id":null}"#
                .to_string()
        })
    }
}

pub fn read_request() -> Option<JsonRpcRequest> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => {
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                return read_request();
            }
            match serde_json::from_str(&trimmed) {
                Ok(req) => Some(req),
                Err(e) => {
                    eprintln!("failed to parse JSON-RPC request: {e}");
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("failed to read from stdin: {e}");
            None
        }
    }
}

pub fn write_response(response: &JsonRpcResponse) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(response.to_string().as_bytes()).ok();
    handle.write_all(b"\n").ok();
    handle.flush().ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonrpc_response_serializes_success() {
        let resp = JsonRpcResponse::success(
            Some(serde_json::json!(1)),
            serde_json::json!({"status": "ok"}),
        );
        let s = resp.to_string();
        assert!(s.contains(r#""jsonrpc":"2.0""#));
        assert!(s.contains(r#""status":"ok""#));
        assert!(!s.contains("error"));
    }

    #[test]
    fn jsonrpc_response_serializes_error() {
        let resp = JsonRpcResponse::error(Some(serde_json::json!(2)), -32601, "not found");
        let s = resp.to_string();
        assert!(s.contains(r#""jsonrpc":"2.0""#));
        assert!(s.contains(r#""code":-32601"#));
        assert!(s.contains(r#""message":"not found""#));
    }
}
