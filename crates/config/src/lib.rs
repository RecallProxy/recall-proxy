use serde::{Deserialize, Serialize};

/// Top-level application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub bind_address: String,
    pub providers: Vec<ProviderConfig>,
    pub routes: Vec<MemoryRouteConfig>,
    pub hindsight_enabled: bool,
}

/// Provider wiring information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub kind: String,
}

/// Serde-serializable provider endpoint configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineProviderConfig {
    pub provider: String,
    pub endpoint: String,
    pub api_key_env: Option<String>,
}

/// Route binding a memory type to a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRouteConfig {
    pub memory_type: String,
    pub provider: EngineProviderConfig,
}

#[cfg(test)]
mod tests {
    use super::{EngineProviderConfig, GatewayConfig, MemoryRouteConfig, ProviderConfig};

    #[test]
    fn gateway_config_collects_provider_configs() {
        let config = GatewayConfig {
            bind_address: "127.0.0.1:8080".to_string(),
            providers: vec![ProviderConfig {
                name: "primary".to_string(),
                kind: "semantic".to_string(),
            }],
            routes: vec![],
            hindsight_enabled: false,
        };

        assert_eq!(config.bind_address, "127.0.0.1:8080");
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].name, "primary");
        assert_eq!(config.providers[0].kind, "semantic");
    }

    #[test]
    fn memory_route_config_is_serializable() {
        let route = MemoryRouteConfig {
            memory_type: "structural".to_string(),
            provider: EngineProviderConfig {
                provider: "my-engine".to_string(),
                endpoint: "http://localhost:9001".to_string(),
                api_key_env: Some("ENGINE_API_KEY".to_string()),
            },
        };

        let json = serde_json::to_string(&route).expect("serialization should succeed");
        assert!(json.contains("structural"));
        assert!(json.contains("my-engine"));
    }
}
