//! MCP server binary entry point.

use recall_proxy_mcp_server::{build_router, state::McpServerState};
use std::net::SocketAddr;
use tracing::info;

fn bind_address() -> SocketAddr {
    let addr = std::env::var("RECALL_PROXY_BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8081".to_string());
    addr.parse()
        .unwrap_or_else(|e| panic!("invalid RECALL_PROXY_BIND_ADDRESS '{}': {e}", addr))
}

fn load_config() -> recall_proxy_config::RecallProxyConfig {
    let config_path = std::env::var("RECALL_PROXY_CONFIG_PATH")
        .unwrap_or_else(|_| "config/examples/multi-engine-orchestration.yaml".to_string());

    let yaml = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        eprintln!(
            "failed to read config at '{}': {}, using defaults",
            config_path, e
        );
        String::new()
    });

    if yaml.is_empty() {
        return recall_proxy_config::RecallProxyConfig::default();
    }

    serde_yaml::from_str(&yaml).unwrap_or_else(|e| {
        panic!("failed to parse config at '{}': {e}", config_path);
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = load_config();
    let addr = bind_address();

    // Build server state from config with startup validation
    let state = match McpServerState::from_config(&config).await {
        Ok(state) => {
            info!("initialized {} memory provider(s) from config", config.providers.len());
            state
        }
        Err(e) => {
            eprintln!("startup validation failed: {e}");
            std::process::exit(1);
        }
    };

    let app = build_router(state);

    info!("starting RecallProxy MCP server on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap_or_else(|e| {
        panic!("failed to bind to {addr}: {e}")
    });
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("server error: {e}"));
}
