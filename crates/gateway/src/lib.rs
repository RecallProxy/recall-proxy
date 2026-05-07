//! HTTP/API-facing gateway orchestration.
//!
//! # Responsibility
//! Hosts request-level orchestration and delegates provider access through
//! `recall-proxy-core` traits.
//!
//! # Public surface
//! - `GatewayRuntime`: runtime boundary for incoming requests.

use recall_proxy_config::GatewayConfig;
use recall_proxy_core::MemoryProvider;

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
    use super::GatewayRuntime;
    use recall_proxy_config::{GatewayConfig, ProviderConfig};
    use recall_proxy_core::MemoryProvider;

    struct StubProvider;

    impl MemoryProvider for StubProvider {
        fn provider_name(&self) -> &'static str {
            "stub"
        }
    }

    #[test]
    fn gateway_runtime_new_stores_config_and_provider() {
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
        assert_eq!(runtime.provider.provider_name(), "stub");
    }
}
