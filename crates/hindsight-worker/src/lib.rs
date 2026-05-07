use async_trait::async_trait;
use recall_proxy_core::memory::MemoryRecord;
use tokio::sync::mpsc;

/// Payload submitted for background hindsight extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestPayload {
    pub interaction_id: String,
    pub raw_text: String,
}

/// Pluggable extractor that converts raw interactions into structured memories.
#[async_trait]
pub trait HindsightExtractor: Send + Sync {
    async fn extract(&self, payload: IngestPayload) -> ExtractedMemories;
}

/// Memories produced by a hindsight extraction step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedMemories {
    pub structural: MemoryRecord,
    pub temporal: MemoryRecord,
}

/// Background pipeline for non-blocking hindsight extraction.
pub struct HindsightPipeline {
    sender: mpsc::Sender<IngestPayload>,
}

impl HindsightPipeline {
    pub fn start<E, F>(extractor: E, mut on_extracted: F) -> Self
    where
        E: HindsightExtractor + 'static,
        F: FnMut(ExtractedMemories) + Send + 'static,
    {
        let (sender, mut receiver) = mpsc::channel::<IngestPayload>(128);

        tokio::spawn(async move {
            while let Some(payload) = receiver.recv().await {
                let extracted = extractor.extract(payload).await;
                on_extracted(extracted);
            }
        });

        Self { sender }
    }

    pub async fn enqueue(
        &self,
        payload: IngestPayload,
    ) -> Result<(), mpsc::error::SendError<IngestPayload>> {
        self.sender.send(payload).await
    }
}

use recall_proxy_config::ProviderConfig;

/// Payload sent to the background worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HindsightJob {
    pub provider: ProviderConfig,
    pub raw_content: String,
}

/// Worker runtime boundary.
pub struct WorkerRuntime;

impl WorkerRuntime {
    pub fn process(&self, _job: HindsightJob) {
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use recall_proxy_core::memory::MemoryRecord;

    use super::{ExtractedMemories, HindsightExtractor, HindsightPipeline, IngestPayload};

    struct MockExtractor;

    #[async_trait]
    impl HindsightExtractor for MockExtractor {
        async fn extract(&self, payload: IngestPayload) -> ExtractedMemories {
            ExtractedMemories {
                structural: MemoryRecord {
                    namespace: payload.interaction_id.clone(),
                    content: format!("structured: {}", payload.raw_text),
                },
                temporal: MemoryRecord {
                    namespace: payload.interaction_id,
                    content: format!("temporal: {}", payload.raw_text),
                },
            }
        }
    }

    #[tokio::test]
    async fn hindsight_pipeline_extracts_in_background() {
        let extracted = Arc::new(Mutex::new(Vec::new()));
        let extracted_collector = Arc::clone(&extracted);

        let pipeline = HindsightPipeline::start(MockExtractor, move |memories| {
            extracted_collector
                .lock()
                .expect("lock should not be poisoned")
                .push(memories);
        });

        pipeline
            .enqueue(IngestPayload {
                interaction_id: "interaction-42".to_string(),
                raw_text: "agent discussed calendar plans".to_string(),
            })
            .await
            .expect("enqueue should succeed");

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let stored = extracted.lock().expect("lock should not be poisoned");
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored[0].structural.content,
            "structured: agent discussed calendar plans"
        );
        assert_eq!(
            stored[0].temporal.content,
            "temporal: agent discussed calendar plans"
        );
    }

    use super::{HindsightJob, WorkerRuntime};
    use recall_proxy_config::ProviderConfig;

    #[test]
    fn hindsight_job_fields_are_preserved() {
        let job = HindsightJob {
            provider: ProviderConfig {
                name: "worker-provider".to_string(),
                kind: "structural".to_string(),
            },
            raw_content: "raw transcript".to_string(),
        };

        assert_eq!(job.provider.name, "worker-provider");
        assert_eq!(job.provider.kind, "structural");
        assert_eq!(job.raw_content, "raw transcript");
    }

    #[test]
    fn worker_process_accepts_job_payload() {
        let worker = WorkerRuntime;
        let job = HindsightJob {
            provider: ProviderConfig {
                name: "worker-provider".to_string(),
                kind: "semantic".to_string(),
            },
            raw_content: "payload".to_string(),
        };

        worker.process(job);
    }
}
