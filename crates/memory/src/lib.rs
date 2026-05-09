//! RecallProxy memory providers.
//!
//! This crate provides both in-memory and SQLite-backed memory engines
//! that implement the `ContextEngine` trait from `recall-proxy-core`.
//!
//! ## Engine Types
//!
//! - **EpisodicEngine** — Session-bound, time-windowed memory storage
//! - **SemanticEngine** — Keyword-based semantic search
//! - **TemporalEngine** — Chronological timeline storage
//! - **StructuralEngine** — Relationship and fact storage
//! - **SqliteMemoryEngine** — SQLite-backed persistence
//!
//! ## Provider Selection Rationale
//!
//! SQLite was chosen as the MVP persistence provider because:
//! - Single-file storage, zero external dependencies beyond the `sqlx` driver
//! - Full ingest (write) and query (read) support
//! - No vendor SDK coupling — uses the standard Rust SQLite driver
//! - Suitable for local development and CI verification
//! - Can be upgraded to a server-backed provider later with minimal gateway changes

pub mod engine;
pub mod engines;
pub mod factory;

pub use engine::{SqliteMemoryEngine, SqliteProviderConfig};
pub use engines::{
    EpisodicEngine, EpisodicEngineConfig, SemanticEngine, SemanticEngineConfig,
    StructuralEngine, StructuralEngineConfig, TemporalEngine, TemporalEngineConfig,
};
