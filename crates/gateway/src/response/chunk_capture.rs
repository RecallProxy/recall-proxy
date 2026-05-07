use std::time::SystemTime;

use super::{FinalizedResponse, FinishReason};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkEvent {
    pub sequence: u64,
    pub delta: String,
    pub received_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkCapture {
    pub response_id: String,
    pub started_at: SystemTime,
    chunks: Vec<ChunkEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkOrderingError {
    pub expected_next: u64,
    pub received: u64,
}

impl ChunkCapture {
    pub fn new(response_id: impl Into<String>, started_at: SystemTime) -> Self {
        Self {
            response_id: response_id.into(),
            started_at,
            chunks: Vec::new(),
        }
    }

    pub fn push_chunk(
        &mut self,
        sequence: u64,
        delta: impl Into<String>,
        received_at: SystemTime,
    ) -> Result<(), ChunkOrderingError> {
        let expected_next = self.chunks.len() as u64;
        if sequence != expected_next {
            return Err(ChunkOrderingError {
                expected_next,
                received: sequence,
            });
        }

        self.chunks.push(ChunkEvent {
            sequence,
            delta: delta.into(),
            received_at,
        });
        Ok(())
    }

    pub fn chunks(&self) -> &[ChunkEvent] {
        &self.chunks
    }

    pub fn assembled_text(&self) -> String {
        self.chunks.iter().map(|chunk| chunk.delta.as_str()).collect()
    }

    pub fn finalize(self, completed_at: SystemTime, finish_reason: FinishReason) -> FinalizedResponse {
        FinalizedResponse::from_capture(self, completed_at, finish_reason)
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::ChunkCapture;
    use crate::response::FinishReason;

    #[test]
    fn captures_chunks_and_assembles_in_order() {
        let mut capture = ChunkCapture::new("resp-1", SystemTime::UNIX_EPOCH);
        capture
            .push_chunk(0, "Hello ", SystemTime::UNIX_EPOCH)
            .expect("first chunk should be accepted");
        capture
            .push_chunk(1, "world", SystemTime::UNIX_EPOCH)
            .expect("second chunk should be accepted");

        assert_eq!(capture.assembled_text(), "Hello world");
        assert_eq!(capture.chunks().len(), 2);

        let finalized = capture.finalize(SystemTime::UNIX_EPOCH, FinishReason::Stop);
        assert_eq!(finalized.content, "Hello world");
        assert_eq!(finalized.chunk_count, 2);
    }

    #[test]
    fn rejects_out_of_order_chunks() {
        let mut capture = ChunkCapture::new("resp-1", SystemTime::UNIX_EPOCH);

        let err = capture
            .push_chunk(2, "skipped chunks", SystemTime::UNIX_EPOCH)
            .expect_err("chunk ordering must be contiguous");

        assert_eq!(err.expected_next, 0);
        assert_eq!(err.received, 2);
    }
}
