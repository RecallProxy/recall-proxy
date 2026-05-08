use axum::{Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing::info;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Deserialize)]
struct IngestRequest {
    interaction_id: String,
    content: String,
}

#[derive(Serialize)]
struct IngestResponse {
    status: &'static str,
    interaction_id: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "recall-proxy-gateway",
    })
}

async fn ingest(Json(payload): Json<IngestRequest>) -> Json<IngestResponse> {
    info!("ingesting interaction: {}", payload.interaction_id);
    Json(IngestResponse {
        status: "stored",
        interaction_id: payload.interaction_id,
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
    let app = Router::new()
        .route("/health", get(health))
        .route("/ingest", post(ingest));

    info!("starting RecallProxy gateway on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap_or_else(|e| {
        panic!("failed to bind to {addr}: {e}")
    });
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("server error: {e}"));
}
