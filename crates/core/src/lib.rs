//! Core abstractions for context memory providers.
//!
//! # Responsibility
//! Defines provider-facing traits and shared request/response types that stay
//! independent from transport/runtime concerns.
//!
//! # Public surface
//! - `MemoryRecord`: a normalized context unit.
//! - `MemoryProvider`: async-compatible provider contract.

/// A normalized memory item produced or consumed by providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub namespace: String,
    pub content: String,
}

/// Contract for pluggable memory providers.
pub trait MemoryProvider {
    fn provider_name(&self) -> &'static str;
}

pub mod context;
pub mod memory;

#[cfg(test)]
mod tests {
    use super::{MemoryProvider, MemoryRecord};

    struct TestProvider;

    impl MemoryProvider for TestProvider {
        fn provider_name(&self) -> &'static str {
            "test-provider"
        }
    }

    #[test]
    fn memory_record_holds_values() {
        let record = MemoryRecord {
            namespace: "session-1".to_string(),
            content: "hello world".to_string(),
        };

        assert_eq!(record.namespace, "session-1");
        assert_eq!(record.content, "hello world");
    }

    #[test]
    fn provider_name_contract_is_callable() {
        let provider = TestProvider;
        assert_eq!(provider.provider_name(), "test-provider");
    }
}
