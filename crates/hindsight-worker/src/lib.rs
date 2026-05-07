//! Async background processing for hindsight extraction flows.
//!
//! # Responsibility
//! Runs non-blocking ingestion and extraction jobs separate from request/response
//! latency paths.
//!
//! # Public surface
//! - `HindsightJob`: minimal job payload contract.
//! - `WorkerRuntime`: worker entrypoint for job processing.

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
        // Placeholder entrypoint for future async execution wiring.
    }
}

#[cfg(test)]
mod tests {
    use super::{HindsightJob, WorkerRuntime};
    use recall_proxy_config::ProviderConfig;

    #[test]
    fn hindsight_job_fields_are_preserved() {
        let job = HindsightJob {
            provider: ProviderConfig {
                id: "worker-provider".to_string(),
                provider_type: recall_proxy_config::ProviderType::Structural,
                enabled: true,
                capabilities: vec![],
                settings: Default::default(),
            },
            raw_content: "raw transcript".to_string(),
        };

        assert_eq!(job.provider.id, "worker-provider");
        assert_eq!(job.raw_content, "raw transcript");
    }

    #[test]
    fn worker_process_accepts_job_payload() {
        let worker = WorkerRuntime;
        let job = HindsightJob {
            provider: ProviderConfig {
                id: "worker-provider".to_string(),
                provider_type: recall_proxy_config::ProviderType::Semantic,
                enabled: true,
                capabilities: vec![],
                settings: Default::default(),
            },
            raw_content: "payload".to_string(),
        };

        worker.process(job);
    }
}
