use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPipelineConfig {
    pub max_parallel_engines: usize,
    pub global_timeout_ms: u64,
    pub fail_open: bool,
    pub engine_configs: Vec<EngineConfig>,
    pub merge_policy: MergePolicyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub name: String,
    pub engine_type: String,
    pub enabled: bool,
    pub timeout_ms: u64,
    pub max_snippets: usize,
    pub min_score: Option<f32>,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePolicyConfig {
    pub precedence: Vec<String>,
    pub synthesis_template: String,
    pub dedupe_strategy: DedupeStrategy,
    pub max_snippets_per_engine: usize,
    pub max_context_tokens: usize,
    pub min_reserved_for_user_prompt_tokens: usize,
    pub min_reserved_for_system_prompt_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DedupeStrategy {
    ExactText,
    NormalizedText,
    SourceReference,
}
