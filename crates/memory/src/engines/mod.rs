//! In-memory memory engine implementations.
//!
//! Provides concrete, in-memory implementations of the `ContextEngine`
//! trait for each memory type: episodic, semantic, temporal, and structural.

pub mod episodic;
pub mod semantic;
pub mod structural;
pub mod temporal;

pub use episodic::{EpisodicEngine, EpisodicEngineConfig};
pub use semantic::{SemanticEngine, SemanticEngineConfig};
pub use structural::{StructuralEngine, StructuralEngineConfig};
pub use temporal::{TemporalEngine, TemporalEngineConfig};
