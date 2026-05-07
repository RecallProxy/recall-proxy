use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    InvalidRequest(String),
    Unauthorized,
    NotFound,
    Timeout,
    ResourceExhausted(String),
    Unavailable(String),
    Internal(String),
}

impl ProviderError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout | Self::Unavailable(_) | Self::ResourceExhausted(_)
        )
    }
}

impl Display for ProviderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "invalid request: {msg}"),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::NotFound => write!(f, "not found"),
            Self::Timeout => write!(f, "timeout"),
            Self::ResourceExhausted(msg) => write!(f, "resource exhausted: {msg}"),
            Self::Unavailable(msg) => write!(f, "temporarily unavailable: {msg}"),
            Self::Internal(msg) => write!(f, "internal provider error: {msg}"),
        }
    }
}

impl Error for ProviderError {}

pub type ProviderResult<T> = Result<T, ProviderError>;
