//! Deterministic merge and synthesis rules for engine snippets.

use std::collections::HashSet;

use recall_proxy_core::context::{
    AssembledContext, ContextRequest, EngineLookupResult, EngineSnippet, TokenBudget,
};

fn dedupe_key(snippet: &EngineSnippet) -> String {
    snippet
        .source_ref
        .clone()
        .unwrap_or_else(|| snippet.text.trim().to_lowercase())
}

pub fn assemble_context(
    _request: &ContextRequest,
    token_budget: &TokenBudget,
    precedence: &[String],
    results: Vec<EngineLookupResult>,
) -> AssembledContext {
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    let mut used_tokens = 0usize;
    let mut dropped_snippets = 0usize;
    let mut output = String::new();
    let max_tokens = token_budget.available_for_context();

    for engine_name in precedence {
        let Some(engine_result) = results.iter().find(|item| item.engine_name == *engine_name) else {
            warnings.push(format!("configured precedence references missing engine '{}'", engine_name));
            continue;
        };

        if engine_result.metrics.failed {
            warnings.push(format!("engine '{}' failed: {:?}", engine_name, engine_result.error));
            continue;
        }

        for snippet in &engine_result.snippets {
            let key = dedupe_key(snippet);
            if !seen.insert(key) {
                dropped_snippets += 1;
                continue;
            }

            if used_tokens + snippet.estimated_tokens > max_tokens {
                dropped_snippets += 1;
                continue;
            }

            if !output.is_empty() {
                output.push_str("\n\n");
            }

            output.push_str(&format!("[{}] {}", engine_name, snippet.text));
            used_tokens += snippet.estimated_tokens;
        }
    }

    AssembledContext {
        synthesized_context: output,
        used_tokens,
        dropped_snippets,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use recall_proxy_core::context::{
        ContextEngineType, ContextRequest, EngineLookupMetrics, EngineLookupResult, EngineSnippet, TokenBudget,
    };

    use super::assemble_context;

    fn request() -> ContextRequest {
        ContextRequest {
            tenant_id: "tenant".to_string(),
            agent_id: "agent".to_string(),
            conversation_id: None,
            user_query: "hello".to_string(),
            max_context_tokens: 128,
            metadata: HashMap::new(),
        }
    }

    fn result(name: &str, snippets: Vec<EngineSnippet>) -> EngineLookupResult {
        EngineLookupResult {
            engine_name: name.to_string(),
            snippets,
            metrics: EngineLookupMetrics {
                latency: Duration::from_millis(5),
                timed_out: false,
                failed: false,
            },
            error: None,
        }
    }

    #[test]
    fn merges_in_precedence_order() {
        let precedence = vec!["semantic".to_string(), "temporal".to_string()];
        let budget = TokenBudget {
            total: 100,
            reserved_for_user_prompt: 10,
            reserved_for_system_prompt: 10,
        };
        let results = vec![
            result(
                "temporal",
                vec![EngineSnippet {
                    engine_name: "temporal".to_string(),
                    engine_type: ContextEngineType::Temporal,
                    rank: 0,
                    text: "second".to_string(),
                    relevance_score: None,
                    estimated_tokens: 5,
                    source_ref: None,
                }],
            ),
            result(
                "semantic",
                vec![EngineSnippet {
                    engine_name: "semantic".to_string(),
                    engine_type: ContextEngineType::Semantic,
                    rank: 0,
                    text: "first".to_string(),
                    relevance_score: None,
                    estimated_tokens: 5,
                    source_ref: None,
                }],
            ),
        ];

        let assembled = assemble_context(&request(), &budget, &precedence, results);
        assert!(assembled.synthesized_context.starts_with("[semantic] first"));
    }

    #[test]
    fn drops_snippets_that_exceed_budget() {
        let precedence = vec!["semantic".to_string()];
        let budget = TokenBudget {
            total: 12,
            reserved_for_user_prompt: 5,
            reserved_for_system_prompt: 5,
        };
        let results = vec![result(
            "semantic",
            vec![EngineSnippet {
                engine_name: "semantic".to_string(),
                engine_type: ContextEngineType::Semantic,
                rank: 0,
                text: "too large".to_string(),
                relevance_score: None,
                estimated_tokens: 4,
                source_ref: None,
            }],
        )];

        let assembled = assemble_context(&request(), &budget, &precedence, results);
        assert_eq!(assembled.used_tokens, 0);
        assert_eq!(assembled.dropped_snippets, 1);
    }
}
