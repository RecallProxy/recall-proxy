pub mod context_pipeline;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayConfig {
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub read_pipelines: Vec<ReadPipeline>,
    #[serde(default)]
    pub write_pipelines: Vec<WritePipeline>,
}

impl GatewayConfig {
    pub fn read_candidates(&self, capability: Capability) -> Vec<&ReadProviderRoute> {
        let mut candidates = self
            .read_pipelines
            .iter()
            .flat_map(|pipeline| pipeline.providers.iter())
            .filter(|route| route.capability == capability && route.enabled)
            .collect::<Vec<_>>();

        // Lowest priority value wins; ties resolve by highest weight first.
        candidates.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| b.weight.cmp(&a.weight))
                .then_with(|| a.provider_id.cmp(&b.provider_id))
        });

        candidates
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub provider_type: ProviderType,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Semantic,
    Structural,
    Temporal,
    Composite,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    SemanticSearch,
    GraphNeighborhood,
    EpisodicTimeline,
    SessionState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadPipeline {
    pub id: String,
    #[serde(default)]
    pub providers: Vec<ReadProviderRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadProviderRoute {
    pub provider_id: String,
    pub capability: Capability,
    #[serde(default = "default_priority")]
    pub priority: u16,
    #[serde(default = "default_weight")]
    pub weight: u16,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WritePipeline {
    pub id: String,
    #[serde(default = "default_trigger")]
    pub trigger: WriteTrigger,
    #[serde(default)]
    pub sinks: Vec<WriteSinkRoute>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WriteTrigger {
    OnRequest,
    OnResponse,
    AsyncHindsight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteSinkRoute {
    pub provider_id: String,
    pub capability: Capability,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_criticality")]
    pub criticality: SinkCriticality,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SinkCriticality {
    Required,
    BestEffort,
}

fn default_enabled() -> bool {
    true
}

fn default_priority() -> u16 {
    100
}

fn default_weight() -> u16 {
    100
}

fn default_trigger() -> WriteTrigger {
    WriteTrigger::OnResponse
}

fn default_criticality() -> SinkCriticality {
    SinkCriticality::BestEffort
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_candidates_are_sorted_by_priority_then_weight() {
        let config = GatewayConfig {
            providers: vec![],
            read_pipelines: vec![ReadPipeline {
                id: "default".to_string(),
                providers: vec![
                    ReadProviderRoute {
                        provider_id: "semantic_secondary".to_string(),
                        capability: Capability::SemanticSearch,
                        priority: 20,
                        weight: 80,
                        enabled: true,
                    },
                    ReadProviderRoute {
                        provider_id: "semantic_primary".to_string(),
                        capability: Capability::SemanticSearch,
                        priority: 10,
                        weight: 50,
                        enabled: true,
                    },
                    ReadProviderRoute {
                        provider_id: "semantic_same_priority_higher_weight".to_string(),
                        capability: Capability::SemanticSearch,
                        priority: 10,
                        weight: 70,
                        enabled: true,
                    },
                ],
            }],
            write_pipelines: vec![],
        };

        let sorted = config.read_candidates(Capability::SemanticSearch);
        let provider_ids = sorted
            .into_iter()
            .map(|route| route.provider_id.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            provider_ids,
            vec![
                "semantic_same_priority_higher_weight".to_string(),
                "semantic_primary".to_string(),
                "semantic_secondary".to_string(),
            ]
        );
    }

    #[test]
    fn read_candidates_filter_out_disabled_routes() {
        let config = GatewayConfig {
            providers: vec![],
            read_pipelines: vec![ReadPipeline {
                id: "default".to_string(),
                providers: vec![
                    ReadProviderRoute {
                        provider_id: "enabled".to_string(),
                        capability: Capability::GraphNeighborhood,
                        priority: 100,
                        weight: 100,
                        enabled: true,
                    },
                    ReadProviderRoute {
                        provider_id: "disabled".to_string(),
                        capability: Capability::GraphNeighborhood,
                        priority: 0,
                        weight: 100,
                        enabled: false,
                    },
                ],
            }],
            write_pipelines: vec![],
        };

        let candidates = config.read_candidates(Capability::GraphNeighborhood);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider_id, "enabled");
    }
}
