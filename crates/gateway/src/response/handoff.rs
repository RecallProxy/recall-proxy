use std::time::SystemTime;

use super::{ChunkEvent, FinalizedResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffPayload {
    pub finalized: FinalizedResponse,
    pub chunks: Vec<ChunkEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffMessage {
    pub payload: HandoffPayload,
    pub enqueued_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffError {
    pub message: String,
}

impl HandoffError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait HandoffSink {
    fn try_enqueue(&self, message: HandoffMessage) -> Result<(), HandoffError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffDisposition {
    Enqueued,
    Dropped { reason: String },
}

pub struct NonBlockingHandoffOrchestrator<S: HandoffSink> {
    sink: S,
}

impl<S: HandoffSink> NonBlockingHandoffOrchestrator<S> {
    pub fn new(sink: S) -> Self {
        Self { sink }
    }

    pub fn handoff(&self, payload: HandoffPayload, now: SystemTime) -> HandoffDisposition {
        let message = HandoffMessage {
            payload,
            enqueued_at: now,
        };

        match self.sink.try_enqueue(message) {
            Ok(()) => HandoffDisposition::Enqueued,
            Err(err) => HandoffDisposition::Dropped {
                reason: err.message,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, time::SystemTime};

    use super::{
        HandoffDisposition, HandoffError, HandoffMessage, HandoffPayload, HandoffSink,
        NonBlockingHandoffOrchestrator,
    };
    use crate::response::{ChunkCapture, FinalizedResponse, FinishReason};

    #[derive(Clone)]
    struct CapturingSink {
        accepted: Rc<RefCell<Vec<HandoffMessage>>>,
        fail_with: Option<String>,
    }

    impl CapturingSink {
        fn success() -> Self {
            Self {
                accepted: Rc::new(RefCell::new(Vec::new())),
                fail_with: None,
            }
        }

        fn failure(message: &str) -> Self {
            Self {
                accepted: Rc::new(RefCell::new(Vec::new())),
                fail_with: Some(message.to_owned()),
            }
        }
    }

    impl HandoffSink for CapturingSink {
        fn try_enqueue(&self, message: HandoffMessage) -> Result<(), HandoffError> {
            if let Some(msg) = &self.fail_with {
                return Err(HandoffError::new(msg.clone()));
            }

            self.accepted.borrow_mut().push(message);
            Ok(())
        }
    }

    fn payload() -> HandoffPayload {
        let mut capture = ChunkCapture::new("resp-3", SystemTime::UNIX_EPOCH);
        capture
            .push_chunk(0, "chunk", SystemTime::UNIX_EPOCH)
            .expect("chunk must be accepted");

        let finalized =
            FinalizedResponse::from_capture(capture.clone(), SystemTime::UNIX_EPOCH, FinishReason::Stop);

        HandoffPayload {
            finalized,
            chunks: capture.chunks().to_vec(),
        }
    }

    #[test]
    fn returns_enqueued_when_sink_accepts() {
        let sink = CapturingSink::success();
        let accepted = sink.accepted.clone();
        let orchestrator = NonBlockingHandoffOrchestrator::new(sink);

        let disposition = orchestrator.handoff(payload(), SystemTime::UNIX_EPOCH);
        assert_eq!(disposition, HandoffDisposition::Enqueued);
        assert_eq!(accepted.borrow().len(), 1);
    }

    #[test]
    fn drops_when_sink_is_unavailable_without_panicking() {
        let sink = CapturingSink::failure("queue unavailable");
        let orchestrator = NonBlockingHandoffOrchestrator::new(sink);

        let disposition = orchestrator.handoff(payload(), SystemTime::UNIX_EPOCH);
        assert_eq!(
            disposition,
            HandoffDisposition::Dropped {
                reason: "queue unavailable".to_owned()
            }
        );
    }
}
