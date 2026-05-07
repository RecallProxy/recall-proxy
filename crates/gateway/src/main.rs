use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::net::SocketAddr;
use tracing::info;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "recall-proxy-gateway",
    })
}

fn bind_address() -> SocketAddr {
    let addr = std::env::var("RECALL_PROXY_BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    addr.parse()
        .unwrap_or_else(|e| panic!("invalid RECALL_PROXY_BIND_ADDRESS '{}': {e}", addr))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let addr = bind_address();
    let app = Router::new().route("/health", get(health));

    info!("starting RecallProxy gateway on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap_or_else(|e| {
        panic!("failed to bind to {addr}: {e}")
    });
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("server error: {e}"));
}
