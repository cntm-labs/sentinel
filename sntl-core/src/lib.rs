//! Core traits and types for the Sentinel ORM.
//!
//! This crate provides the foundational abstractions used by the `sntl` ORM:
//! Model traits, query building primitives, relation type-states, and error types.

/// Sentinel core — compile-time guarded ORM primitives.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
