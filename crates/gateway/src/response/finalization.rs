use std::time::SystemTime;

use super::ChunkCapture;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCall,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizedResponse {
    pub response_id: String,
    pub content: String,
    pub chunk_count: usize,
    pub started_at: SystemTime,
    pub completed_at: SystemTime,
    pub finish_reason: FinishReason,
}

impl FinalizedResponse {
    pub fn from_capture(
        capture: ChunkCapture,
        completed_at: SystemTime,
        finish_reason: FinishReason,
    ) -> Self {
        let response_id = capture.response_id.clone();
        let started_at = capture.started_at;
        let content = capture.assembled_text();
        let chunk_count = capture.chunks().len();

        Self {
            response_id,
            content,
            chunk_count,
            started_at,
            completed_at,
            finish_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::{FinalizedResponse, FinishReason};
    use crate::response::ChunkCapture;

    #[test]
    fn finalization_carries_response_metadata() {
        let mut capture = ChunkCapture::new("resp-2", SystemTime::UNIX_EPOCH);
        capture
            .push_chunk(0, "abc", SystemTime::UNIX_EPOCH)
            .expect("chunk must be accepted");

        let finalized =
            FinalizedResponse::from_capture(capture, SystemTime::UNIX_EPOCH, FinishReason::Length);

        assert_eq!(finalized.response_id, "resp-2");
        assert_eq!(finalized.content, "abc");
        assert_eq!(finalized.chunk_count, 1);
        assert_eq!(finalized.finish_reason, FinishReason::Length);
    }
}
