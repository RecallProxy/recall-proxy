use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryEngineType {
    Structural,
    Semantic,
    Temporal,
    Graph,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractError {
    field: &'static str,
    reason: &'static str,
}

impl ContractError {
    fn empty(field: &'static str) -> Self {
        Self {
            field,
            reason: "must not be empty",
        }
    }
}

impl Display for ContractError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.field, self.reason)
    }
}

impl Error for ContractError {}

macro_rules! id_type {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, ContractError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ContractError::empty($field));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

id_type!(EventId, "event_id");
id_type!(HandoffId, "handoff_id");
id_type!(CorrelationId, "correlation_id");
id_type!(TraceId, "trace_id");
id_type!(SpanId, "span_id");
id_type!(DedupeKey, "dedupe_key");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub correlation_id: CorrelationId,
    pub causation_event_id: Option<EventId>,
    pub parent_span_id: Option<SpanId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryMetadata {
    pub producer: String,
    pub produced_at_ms: u128,
    pub target_engine: MemoryEngineType,
    pub attempt: u32,
}

impl DeliveryMetadata {
    pub fn new(
        producer: impl Into<String>,
        target_engine: MemoryEngineType,
        attempt: u32,
    ) -> Result<Self, ContractError> {
        let producer = producer.into();
        if producer.trim().is_empty() {
            return Err(ContractError::empty("producer"));
        }
        Ok(Self {
            producer,
            produced_at_ms: now_epoch_ms(),
            target_engine,
            attempt,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffEnvelope<TPayload> {
    pub event_id: EventId,
    pub handoff_id: HandoffId,
    pub event_type: String,
    pub dedupe_key: DedupeKey,
    pub trace: TraceContext,
    pub delivery: DeliveryMetadata,
    pub payload: TPayload,
}

impl<TPayload> HandoffEnvelope<TPayload> {
    pub fn new(
        event_id: EventId,
        handoff_id: HandoffId,
        event_type: impl Into<String>,
        trace: TraceContext,
        delivery: DeliveryMetadata,
        payload: TPayload,
    ) -> Result<Self, ContractError> {
        let event_type = event_type.into();
        if event_type.trim().is_empty() {
            return Err(ContractError::empty("event_type"));
        }

        let dedupe_key = DedupeKey::parse(format!(
            "{}:{}:{}",
            handoff_id.as_str(),
            event_type,
            event_id.as_str()
        ))?;

        Ok(Self {
            event_id,
            handoff_id,
            event_type,
            dedupe_key,
            trace,
            delivery,
            payload,
        })
    }
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trace() -> TraceContext {
        TraceContext {
            trace_id: TraceId::parse("trace-1").expect("trace id should be valid"),
            span_id: SpanId::parse("span-1").expect("span id should be valid"),
            correlation_id: CorrelationId::parse("corr-1")
                .expect("correlation id should be valid"),
            causation_event_id: Some(EventId::parse("cause-1").expect("event id should be valid")),
            parent_span_id: Some(SpanId::parse("parent-1").expect("span id should be valid")),
        }
    }

    #[test]
    fn id_types_reject_empty_values() {
        let err = EventId::parse("  ").expect_err("empty id should fail");
        assert_eq!(err.to_string(), "event_id must not be empty");
    }

    #[test]
    fn delivery_metadata_sets_timestamp() {
        let metadata = DeliveryMetadata::new("gateway", MemoryEngineType::Temporal, 1)
            .expect("metadata should be valid");

        assert_eq!(metadata.producer, "gateway");
        assert!(metadata.produced_at_ms > 0);
        assert_eq!(metadata.target_engine, MemoryEngineType::Temporal);
    }

    #[test]
    fn envelope_builds_dedupe_key_from_contract_fields() {
        let envelope = HandoffEnvelope::new(
            EventId::parse("evt-1").expect("event id should be valid"),
            HandoffId::parse("handoff-1").expect("handoff id should be valid"),
            "ingest.received",
            sample_trace(),
            DeliveryMetadata::new("gateway", MemoryEngineType::Structural, 2)
                .expect("metadata should be valid"),
            "payload",
        )
        .expect("envelope should be valid");

        assert_eq!(envelope.dedupe_key.as_str(), "handoff-1:ingest.received:evt-1");
    }
}
