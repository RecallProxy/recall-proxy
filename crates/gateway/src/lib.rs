//! HTTP/API-facing gateway orchestration.
//!
//! # Responsibility
//! Hosts request-level orchestration and delegates provider access through
//! `recall-proxy-core` traits.
//!
//! # Public surface
//! - `GatewayRuntime`: runtime boundary for incoming requests.
//! - `context_assembly`: deterministic snippet merging.
//! - `request`: `ContextEngineProvider`, `ContextAssembler`, orchestration.
//! - `response`: streaming response capture and handoff orchestration.
//! - `orchestrator`: async orchestrator with parallel ingest and context assembly.

pub mod context_assembly;
pub mod request;
pub mod response;
pub mod orchestrator;

use recall_proxy_config::GatewayConfig;
use recall_proxy_core::memory::MemoryProvider;

/// Runtime entrypoint for API request orchestration.
pub struct GatewayRuntime<P: MemoryProvider> {
    pub config: GatewayConfig,
    pub provider: P,
}

impl<P: MemoryProvider> GatewayRuntime<P> {
    pub fn new(config: GatewayConfig, provider: P) -> Self {
        Self { config, provider }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::time::Duration;

    use recall_proxy_core::memory::{CapabilityDescriptor, MemoryKind, MemoryProvider, ProviderMetadata};
    use recall_proxy_core::error::ProviderResult;

    use super::GatewayRuntime;
    use recall_proxy_config::{GatewayConfig, ProviderConfig};

    struct StubProvider;

    impl MemoryProvider for StubProvider {
        fn metadata(&self) -> ProviderMetadata {
            ProviderMetadata {
                provider_id: Cow::Borrowed("stub"),
                version: Cow::Borrowed("0.1.0"),
                capabilities: vec![CapabilityDescriptor {
                    kind: MemoryKind::Temporal,
                    supports_ingest: true,
                    supports_query: true,
                    supports_streaming: false,
                    max_batch_size: None,
                }],
            }
        }

        async fn healthcheck(&self, _timeout: Duration) -> ProviderResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn gateway_runtime_new_stores_config_and_provider() {
        let config = GatewayConfig {
            bind_address: "0.0.0.0:9000".to_string(),
            providers: vec![ProviderConfig {
                name: "stub-provider".to_string(),
                kind: "temporal".to_string(),
            }],
        };

        let runtime = GatewayRuntime::new(config, StubProvider);

        assert_eq!(runtime.config.bind_address, "0.0.0.0:9000");
        assert_eq!(runtime.config.providers.len(), 1);
        assert_eq!(runtime.provider.metadata().provider_id, "stub");
    }
}
