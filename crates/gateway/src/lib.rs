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
        use recall_proxy_config::ProviderType;

        let config = GatewayConfig {
            providers: vec![ProviderConfig {
                id: "stub-provider".to_string(),
                provider_type: ProviderType::Temporal,
                enabled: true,
                capabilities: vec![],
                settings: Default::default(),
            }],
            read_pipelines: vec![],
            write_pipelines: vec![],
            bind_address: String::new(),
        };

        let runtime = GatewayRuntime::new(config, StubProvider);

        assert_eq!(runtime.config.providers.len(), 1);
        assert_eq!(runtime.provider.metadata().provider_id, "stub");
    }
}
