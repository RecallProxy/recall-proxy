//! Configuration schema and loading contracts.
//!
//! # Responsibility
//! Holds runtime-agnostic configuration types used to wire providers and
//! gateway behavior.
//!
//! # Public surface
//! - `GatewayConfig`: top-level gateway settings.
//! - `ProviderConfig`: provider metadata used by orchestration.

/// Top-level application configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    pub bind_address: String,
    pub providers: Vec<ProviderConfig>,
}

/// Provider wiring information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub name: String,
    pub kind: String,
}

#[cfg(test)]
mod tests {
    use super::{GatewayConfig, ProviderConfig};

    #[test]
    fn gateway_config_collects_provider_configs() {
        let config = GatewayConfig {
            bind_address: "127.0.0.1:8080".to_string(),
            providers: vec![ProviderConfig {
                name: "primary".to_string(),
                kind: "semantic".to_string(),
            }],
        };

        assert_eq!(config.bind_address, "127.0.0.1:8080");
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].name, "primary");
        assert_eq!(config.providers[0].kind, "semantic");
    }
}
