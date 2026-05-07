pub mod context_assembly;
pub mod request;

use recall_proxy_config::GatewayConfig;

pub struct GatewayRuntime {
    pub config: GatewayConfig,
}

impl GatewayRuntime {
    pub fn new(config: GatewayConfig) -> Self {
        Self { config }
    }
}

#[cfg(test)]
mod tests {
    use super::GatewayRuntime;
    use recall_proxy_config::{Capability, GatewayConfig, ProviderConfig, ProviderType, ReadPipeline, ReadProviderRoute};

    #[test]
    fn gateway_runtime_new_stores_config() {
        let config = GatewayConfig {
            providers: vec![ProviderConfig {
                id: "stub".to_string(),
                provider_type: ProviderType::Semantic,
                enabled: true,
                capabilities: vec![Capability::SemanticSearch],
                settings: Default::default(),
            }],
            read_pipelines: vec![ReadPipeline {
                id: "default".to_string(),
                providers: vec![ReadProviderRoute {
                    provider_id: "stub".to_string(),
                    capability: Capability::SemanticSearch,
                    priority: 10,
                    weight: 100,
                    enabled: true,
                }],
            }],
            write_pipelines: vec![],
        };

        let runtime = GatewayRuntime::new(config);
        assert_eq!(runtime.config.providers.len(), 1);
    }
}
