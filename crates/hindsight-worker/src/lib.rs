
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawEvent {
    pub event_id: String,
    pub tenant_id: String,
    pub content: String,
    pub timestamp_ms: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedEvent {
    pub event_id: String,
    pub tenant_id: String,
    pub normalized_content: String,
    pub timestamp_ms: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractionResult {
    pub structural_facts: Vec<String>,
    pub semantic_blobs: Vec<String>,
    pub temporal_notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryType {
    Structural,
    Semantic,
    Temporal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderPayload {
    pub event_id: String,
    pub tenant_id: String,
    pub memory_type: MemoryType,
    pub body: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WorkerError {
    #[error("event content is empty after normalization")]
    EmptyContent,
    #[error("extractor failed after {attempts} attempts: {last_error}")]
    ExtractionFailed { attempts: usize, last_error: String },
    #[error("provider {provider} failed after {attempts} attempts: {last_error}")]
    ProviderFailed {
        provider: String,
        attempts: usize,
        last_error: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalSignal {
    Completed {
        event_id: String,
        provider_writes: usize,
        extraction_attempts: usize,
    },
    Failed {
        event_id: String,
        reason: String,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct RetryPolicy {
    pub max_attempts: usize,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 3 }
    }
}

#[async_trait]
pub trait Extractor: Send + Sync {
    async fn extract(&self, event: &NormalizedEvent) -> Result<ExtractionResult, String>;
}

#[async_trait]
pub trait MemoryProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn memory_type(&self) -> MemoryType;
    async fn write(&self, payload: ProviderPayload) -> Result<(), String>;
}

pub struct HindsightWorker {
    extractor: Arc<dyn Extractor>,
    providers: Vec<Arc<dyn MemoryProvider>>,
    retry_policy: RetryPolicy,
}

impl HindsightWorker {
    pub fn new(
        extractor: Arc<dyn Extractor>,
        providers: Vec<Arc<dyn MemoryProvider>>,
        retry_policy: RetryPolicy,
    ) -> Self {
        Self {
            extractor,
            providers,
            retry_policy,
        }
    }

    pub fn normalize(raw: RawEvent) -> Result<NormalizedEvent, WorkerError> {
        let collapsed = raw.content.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            return Err(WorkerError::EmptyContent);
        }

        Ok(NormalizedEvent {
            event_id: raw.event_id,
            tenant_id: raw.tenant_id,
            normalized_content: collapsed,
            timestamp_ms: raw.timestamp_ms,
            metadata: raw.metadata,
        })
    }

    pub async fn run(&self, raw: RawEvent) -> TerminalSignal {
        let event_id = raw.event_id.clone();

        match self.run_inner(raw).await {
            Ok((provider_writes, extraction_attempts)) => TerminalSignal::Completed {
                event_id,
                provider_writes,
                extraction_attempts,
            },
            Err(err) => TerminalSignal::Failed {
                event_id,
                reason: err.to_string(),
            },
        }
    }

    async fn run_inner(&self, raw: RawEvent) -> Result<(usize, usize), WorkerError> {
        let normalized = Self::normalize(raw)?;
        let (extraction, extraction_attempts) = self.extract_with_retries(&normalized).await?;
        let write_count = self
            .fanout_to_providers(normalized, extraction)
            .await?;

        Ok((write_count, extraction_attempts))
    }

    async fn extract_with_retries(
        &self,
        event: &NormalizedEvent,
    ) -> Result<(ExtractionResult, usize), WorkerError> {
        let mut last_error = String::new();

        for attempt in 1..=self.retry_policy.max_attempts {
            match self.extractor.extract(event).await {
                Ok(result) => return Ok((result, attempt)),
                Err(err) => {
                    last_error = err;
                }
            }
        }

        Err(WorkerError::ExtractionFailed {
            attempts: self.retry_policy.max_attempts,
            last_error,
        })
    }

    async fn fanout_to_providers(
        &self,
        event: NormalizedEvent,
        extraction: ExtractionResult,
    ) -> Result<usize, WorkerError> {
        let payloads = self.build_payloads(&event, &extraction);

        let futures = self
            .providers
            .iter()
            .map(|provider| {
                let payload = payloads
                    .iter()
                    .find(|candidate| candidate.memory_type == provider.memory_type())
                    .expect("payload for provider memory type")
                    .clone();

                async move {
                    self.write_with_retries(provider.clone(), payload)
                        .await
                }
            })
            .collect::<Vec<_>>();

        let results = join_all(futures).await;
        for result in results {
            result?;
        }

        Ok(self.providers.len())
    }

    fn build_payloads(
        &self,
        event: &NormalizedEvent,
        extraction: &ExtractionResult,
    ) -> Vec<ProviderPayload> {
        vec![
            ProviderPayload {
                event_id: event.event_id.clone(),
                tenant_id: event.tenant_id.clone(),
                memory_type: MemoryType::Structural,
                body: extraction.structural_facts.join("\n"),
            },
            ProviderPayload {
                event_id: event.event_id.clone(),
                tenant_id: event.tenant_id.clone(),
                memory_type: MemoryType::Semantic,
                body: extraction.semantic_blobs.join("\n"),
            },
            ProviderPayload {
                event_id: event.event_id.clone(),
                tenant_id: event.tenant_id.clone(),
                memory_type: MemoryType::Temporal,
                body: extraction.temporal_notes.join("\n"),
            },
        ]
    }

    async fn write_with_retries(
        &self,
        provider: Arc<dyn MemoryProvider>,
        payload: ProviderPayload,
    ) -> Result<(), WorkerError> {
        let mut last_error = String::new();

        for _ in 1..=self.retry_policy.max_attempts {
            match provider.write(payload.clone()).await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    last_error = err;
                }
            }
        }

        Err(WorkerError::ProviderFailed {
            provider: provider.name().to_string(),
            attempts: self.retry_policy.max_attempts,
            last_error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    struct MockExtractor {
        failures_before_success: AtomicUsize,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl Extractor for MockExtractor {
        async fn extract(&self, _: &NormalizedEvent) -> Result<ExtractionResult, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let remaining = self.failures_before_success.load(Ordering::SeqCst);
            if remaining > 0 {
                self.failures_before_success.fetch_sub(1, Ordering::SeqCst);
                return Err("temporary extraction error".to_string());
            }

            Ok(ExtractionResult {
                structural_facts: vec!["node:a -> node:b".to_string()],
                semantic_blobs: vec!["user discussed retries".to_string()],
                temporal_notes: vec!["2026-05-07 ingestion".to_string()],
            })
        }
    }

    struct MockProvider {
        provider_name: &'static str,
        target_type: MemoryType,
        failures_before_success: AtomicUsize,
        payloads: Mutex<Vec<ProviderPayload>>,
    }

    #[async_trait]
    impl MemoryProvider for MockProvider {
        fn name(&self) -> &'static str {
            self.provider_name
        }

        fn memory_type(&self) -> MemoryType {
            self.target_type.clone()
        }

        async fn write(&self, payload: ProviderPayload) -> Result<(), String> {
            let remaining = self.failures_before_success.load(Ordering::SeqCst);
            if remaining > 0 {
                self.failures_before_success.fetch_sub(1, Ordering::SeqCst);
                return Err("provider transient failure".to_string());
            }

            self.payloads.lock().unwrap().push(payload);
            Ok(())
        }
    }

    fn sample_event() -> RawEvent {
        RawEvent {
            event_id: "evt-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            content: "  hello    from   hindsight   worker ".to_string(),
            timestamp_ms: 42,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn normalizes_content_before_processing() {
        let normalized = HindsightWorker::normalize(sample_event()).expect("normalize succeeds");
        assert_eq!(normalized.normalized_content, "hello from hindsight worker");
    }

    #[tokio::test]
    async fn retries_extraction_then_fanouts_to_all_providers() {
        let extractor = Arc::new(MockExtractor {
            failures_before_success: AtomicUsize::new(1),
            calls: AtomicUsize::new(0),
        });

        let structural = Arc::new(MockProvider {
            provider_name: "structural",
            target_type: MemoryType::Structural,
            failures_before_success: AtomicUsize::new(0),
            payloads: Mutex::new(Vec::new()),
        });
        let semantic = Arc::new(MockProvider {
            provider_name: "semantic",
            target_type: MemoryType::Semantic,
            failures_before_success: AtomicUsize::new(0),
            payloads: Mutex::new(Vec::new()),
        });
        let temporal = Arc::new(MockProvider {
            provider_name: "temporal",
            target_type: MemoryType::Temporal,
            failures_before_success: AtomicUsize::new(0),
            payloads: Mutex::new(Vec::new()),
        });

        let worker = HindsightWorker::new(
            extractor.clone(),
            vec![structural.clone(), semantic.clone(), temporal.clone()],
            RetryPolicy { max_attempts: 3 },
        );

        let signal = worker.run(sample_event()).await;

        assert_eq!(
            signal,
            TerminalSignal::Completed {
                event_id: "evt-1".to_string(),
                provider_writes: 3,
                extraction_attempts: 2,
            }
        );
        assert_eq!(extractor.calls.load(Ordering::SeqCst), 2);
        assert_eq!(structural.payloads.lock().unwrap().len(), 1);
        assert_eq!(semantic.payloads.lock().unwrap().len(), 1);
        assert_eq!(temporal.payloads.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn retries_provider_writes_and_emits_failure_terminal_signal() {
        let extractor = Arc::new(MockExtractor {
            failures_before_success: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
        });

        let broken_structural = Arc::new(MockProvider {
            provider_name: "structural",
            target_type: MemoryType::Structural,
            failures_before_success: AtomicUsize::new(99),
            payloads: Mutex::new(Vec::new()),
        });

        let worker = HindsightWorker::new(
            extractor,
            vec![broken_structural],
            RetryPolicy { max_attempts: 2 },
        );

        let signal = worker.run(sample_event()).await;
        match signal {
            TerminalSignal::Failed { reason, .. } => {
                assert!(reason.contains("provider structural failed after 2 attempts"));
            }
            _ => panic!("expected failure signal"),
        }
    }
}
