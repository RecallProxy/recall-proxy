//! Core contracts for RecallProxy provider integrations.
//!
//! This crate surface is intentionally minimal and focuses on shared types and
//! provider traits used by higher-level orchestration layers.

pub mod contracts;
pub mod context;
pub mod error;
pub mod events;
pub mod gateway_types;
pub mod memory;
