use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::SystemTime;

pub use crate::context::ContextEngineType;

#[deprecated(
    since = "0.1.0",
    note = "use ContextEngineType from the context module instead"
)]
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryType {
    Structural,
    Temporal,
    Semantic,
}

#[allow(deprecated)]
impl MemoryType {
    fn as_context_engine_type(&self) -> ContextEngineType {
        match self {
            MemoryType::Structural => ContextEngineType::Structural,
            MemoryType::Temporal => ContextEngineType::Temporal,
            MemoryType::Semantic => ContextEngineType::Semantic,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPayload {
    pub session_id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct MemoryQuery {
    pub session_id: String,
    pub prompt: String,
    pub max_results: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextSnippet {
    pub source: String,
    pub engine_type: ContextEngineType,
    pub content: String,
    pub score: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct IngestReceipt {
    pub ingested_at: SystemTime,
    pub scheduled_hindsight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError {
    message: String,
}

impl EngineError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for EngineError {}
