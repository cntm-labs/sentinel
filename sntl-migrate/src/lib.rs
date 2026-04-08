//! Schema diff and migration generation for the Sentinel ORM.
//!
//! This crate will provide automatic migration generation by diffing
//! your Rust model definitions against the live database schema.

/// Sentinel migrate — schema diff and migration generation.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
