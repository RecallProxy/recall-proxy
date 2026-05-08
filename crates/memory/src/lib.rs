//! SQLite-backed memory provider for RecallProxy.
//!
//! This crate provides a concrete, file-based memory engine that implements
//! the `ContextEngine` trait from `recall-proxy-core`. It is the smallest
//! practical MVP that qualifies as a "live backend" — a real database
//! requiring no external server process.
//!
//! ## Provider Selection Rationale
//!
//! SQLite was chosen as the MVP provider because:
//! - Single-file storage, zero external dependencies beyond the `sqlx` driver
//! - Full ingest (write) and query (read) support
//! - No vendor SDK coupling — uses the standard Rust SQLite driver
//! - Suitable for local development and CI verification
//! - Can be upgraded to a server-backed provider later with minimal gateway changes

pub mod engine;
pub mod factory;

pub use engine::{SqliteMemoryEngine, SqliteProviderConfig};
