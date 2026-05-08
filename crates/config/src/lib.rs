pub mod context_pipeline;

use std::collections::BTreeMap;

use recall_proxy_core::context::RetrievalIntent;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Legacy types (kept for backward compatibility)
// ---------------------------------------------------------------------------

/// Top-level application configuration.
#[deprecated(
    since = "0.1.0",
    note = "use RecallProxyConfig which supports the full provider-driven schema"
)]
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    pub bind_address: String,
    pub providers: Vec<ProviderConfig>,
}

/// Provider wiring information.
#[deprecated(
    since = "0.1.0",
    note = "use ProviderRegistration which supports the full provider-driven schema"
)]
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub name: String,
    pub kind: String,
}

// ---------------------------------------------------------------------------
// Canonical provider-driven config model (matches YAML examples)
// ---------------------------------------------------------------------------

/// Top-level RecallProxy configuration as expressed in YAML examples.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecallProxyConfig {
    #[serde(default)]
    pub providers: Vec<ProviderRegistration>,

    #[serde(default)]
    pub read_pipelines: Vec<ReadPipeline>,

    #[serde(default)]
    pub write_pipelines: Vec<WritePipeline>,

    #[serde(default)]
    pub bind_address: Option<String>,
}

/// A registered memory provider available to the gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRegistration {
    pub id: String,

    #[serde(rename = "provider_type")]
    pub provider_type: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub capabilities: Vec<String>,

    #[serde(default)]
    pub settings: BTreeMap<String, String>,
}

/// A request-time retrieval pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadPipeline {
    pub id: String,

    #[serde(default)]
    pub providers: Vec<ReadProviderRoute>,
}

/// A single route within a read pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadProviderRoute {
    #[serde(rename = "provider_id")]
    pub provider_id: String,

    pub capability: String,

    /// Optional retrieval intent filter. When present, this route only
    /// serves requests whose intent matches (or is a superset of) this value.
    #[serde(default)]
    pub intent: Option<RetrievalIntent>,

    #[serde(default)]
    pub priority: u32,

    #[serde(default)]
    pub weight: u32,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// A response-time or background write pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritePipeline {
    pub id: String,

    pub trigger: WriteTrigger,

    #[serde(default)]
    pub sinks: Vec<WriteSink>,
}

/// Triggers that activate a write pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteTrigger {
    OnRequest,
    OnResponse,
    AsyncHindsight,
}

/// A single sink within a write pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteSink {
    #[serde(rename = "provider_id")]
    pub provider_id: String,

    pub capability: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub criticality: WriteCriticality,
}

/// Whether a write sink failure fails the stage or is logged and ignored.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteCriticality {
    #[default]
    Required,
    BestEffort,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Helpers for migrating from GatewayConfig → RecallProxyConfig
// ---------------------------------------------------------------------------

impl GatewayConfig {
    /// Convert the legacy config into the canonical model.
    ///
    /// This is a best-effort migration: settings and capabilities are left
    /// empty because the legacy types do not carry that information.
    pub fn to_canonical(&self) -> RecallProxyConfig {
        RecallProxyConfig {
            bind_address: Some(self.bind_address.clone()),
            providers: self
                .providers
                .iter()
                .map(|p| ProviderRegistration {
                    id: p.name.clone(),
                    provider_type: p.kind.clone(),
                    enabled: true,
                    capabilities: Vec::new(),
                    settings: BTreeMap::new(),
                })
                .collect(),
            read_pipelines: Vec::new(),
            write_pipelines: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

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

    #[test]
    fn legacy_gateway_config_migrates_to_canonical() {
        let legacy = GatewayConfig {
            bind_address: "0.0.0.0:3000".to_string(),
            providers: vec![
                ProviderConfig {
                    name: "semantic_local".to_string(),
                    kind: "semantic".to_string(),
                },
                ProviderConfig {
                    name: "graph_engine".to_string(),
                    kind: "structural".to_string(),
                },
            ],
        };

        let canonical = legacy.to_canonical();

        assert_eq!(canonical.bind_address, Some("0.0.0.0:3000".to_string()));
        assert_eq!(canonical.providers.len(), 2);
        assert_eq!(canonical.providers[0].id, "semantic_local");
        assert_eq!(canonical.providers[0].provider_type, "semantic");
        assert_eq!(canonical.providers[1].id, "graph_engine");
        assert_eq!(canonical.providers[1].provider_type, "structural");
    }

    #[test]
    fn simple_single_engine_yaml_deserializes() {
        let yaml = r#"
providers:
  - id: semantic_local
    provider_type: semantic
    enabled: true
    capabilities:
      - semantic_search
    settings:
      endpoint: http://localhost:6333
      collection: recall_embeddings

read_pipelines:
  - id: default
    providers:
      - provider_id: semantic_local
        capability: semantic_search
        priority: 10
        weight: 100
        enabled: true

write_pipelines:
  - id: response_archive
    trigger: on_response
    sinks:
      - provider_id: semantic_local
        capability: semantic_search
        criticality: required
        enabled: true
"#;

        let config: RecallProxyConfig =
            serde_yaml::from_str(yaml).expect("simple-single-engine should deserialize");

        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].id, "semantic_local");
        assert_eq!(config.providers[0].provider_type, "semantic");
        assert_eq!(config.providers[0].capabilities.len(), 1);
        assert_eq!(config.providers[0].capabilities[0], "semantic_search");
        assert_eq!(
            config.providers[0].settings.get("endpoint").map(|s| s.as_str()),
            Some("http://localhost:6333")
        );

        assert_eq!(config.read_pipelines.len(), 1);
        assert_eq!(config.read_pipelines[0].id, "default");
        assert_eq!(config.read_pipelines[0].providers.len(), 1);
        assert_eq!(
            config.read_pipelines[0].providers[0].provider_id,
            "semantic_local"
        );

        assert_eq!(config.write_pipelines.len(), 1);
        assert_eq!(
            config.write_pipelines[0].trigger,
            WriteTrigger::OnResponse
        );
        assert_eq!(config.write_pipelines[0].sinks.len(), 1);
        assert_eq!(
            config.write_pipelines[0].sinks[0].provider_id,
            "semantic_local"
        );
        assert_eq!(
            config.write_pipelines[0].sinks[0].criticality,
            WriteCriticality::Required
        );
    }

    #[test]
    fn multi_engine_orchestration_yaml_deserializes() {
        let yaml = r#"
providers:
  - id: semantic_primary
    provider_type: semantic
    capabilities:
      - semantic_search
    settings:
      endpoint: https://semantic-primary.internal
      index: customer-support

  - id: graph_engine
    provider_type: structural
    capabilities:
      - graph_neighborhood
    settings:
      endpoint: https://graph.internal
      namespace: support-relations

read_pipelines:
  - id: llm_request
    providers:
      - provider_id: semantic_primary
        capability: semantic_search
        priority: 10
        weight: 100

write_pipelines:
  - id: response_persist
    trigger: on_response
    sinks:
      - provider_id: semantic_primary
        capability: semantic_search
        criticality: best_effort
"#;

        let config: RecallProxyConfig =
            serde_yaml::from_str(yaml).expect("multi-engine should deserialize");

        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.read_pipelines.len(), 1);
        assert_eq!(config.write_pipelines.len(), 1);
        assert_eq!(
            config.write_pipelines[0].sinks[0].criticality,
            WriteCriticality::BestEffort
        );
    }

    #[test]
    fn minimal_config_defaults_are_valid() {
        let yaml = r#"{}"#;
        let config: RecallProxyConfig =
            serde_yaml::from_str(yaml).expect("empty config should deserialize");

        assert!(config.providers.is_empty());
        assert!(config.read_pipelines.is_empty());
        assert!(config.write_pipelines.is_empty());
        assert!(config.bind_address.is_none());
    }

    #[test]
    fn read_pipeline_with_intent_filter_deserializes() {
        let yaml = r#"
providers:
  - id: episodic_store
    provider_type: episodic
    capabilities:
      - episodic_retrieve

read_pipelines:
  - id: episodic_lookup
    providers:
      - provider_id: episodic_store
        capability: episodic_retrieve
        intent: episodic
        priority: 5
        enabled: true
"#;

        let config: RecallProxyConfig =
            serde_yaml::from_str(yaml).expect("intent-filtered pipeline should deserialize");

        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].provider_type, "episodic");
        assert_eq!(config.read_pipelines.len(), 1);
        assert_eq!(config.read_pipelines[0].id, "episodic_lookup");
        assert_eq!(config.read_pipelines[0].providers.len(), 1);
        assert_eq!(
            config.read_pipelines[0].providers[0].intent,
            Some(RetrievalIntent::Episodic)
        );
    }

    #[test]
    fn read_pipeline_without_intent_filter_defaults_to_none() {
        let yaml = r#"
providers:
  - id: semantic_local
    provider_type: semantic
    capabilities:
      - semantic_search

read_pipelines:
  - id: default
    providers:
      - provider_id: semantic_local
        capability: semantic_search
"#;

        let config: RecallProxyConfig =
            serde_yaml::from_str(yaml).expect("pipeline without intent should deserialize");

        assert_eq!(
            config.read_pipelines[0].providers[0].intent,
            None
        );
    }

    #[test]
    fn mixed_intent_serves_all_engine_types() {
        let yaml = r#"
providers:
  - id: semantic_local
    provider_type: semantic
    capabilities:
      - semantic_search
  - id: graph_engine
    provider_type: structural
    capabilities:
      - graph_neighborhood
  - id: temporal_store
    provider_type: temporal
    capabilities:
      - timeline_query

read_pipelines:
  - id: mixed_lookup
    providers:
      - provider_id: semantic_local
        capability: semantic_search
        intent: mixed
        priority: 1
      - provider_id: graph_engine
        capability: graph_neighborhood
        intent: mixed
        priority: 2
      - provider_id: temporal_store
        capability: timeline_query
        intent: mixed
        priority: 3
"#;

        let config: RecallProxyConfig =
            serde_yaml::from_str(yaml).expect("mixed intent pipeline should deserialize");

        assert_eq!(config.read_pipelines.len(), 1);
        assert_eq!(config.read_pipelines[0].providers.len(), 3);
        for route in &config.read_pipelines[0].providers {
            assert_eq!(route.intent, Some(RetrievalIntent::Mixed));
        }
    }
}
