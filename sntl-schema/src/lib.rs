//! Schema analysis and cache library shared by `sntl-macros` and `sntl-cli`.
//!
//! Modules:
//! - [`config`]: parse `sentinel.toml` + env overrides.
//! - [`schema`]: typed model of `schema.toml` (tables, columns, enums, composites).
//! - [`cache`]: read/write `.sentinel/` directory.
//! - [`normalize`]: deterministic SQL normalization + hashing.
//! - [`parser`]: sqlparser-rs wrapper.
//! - [`scope`]: FROM/JOIN scope resolution and column origins.
//! - [`nullable`]: nullability inference engine.
//! - [`resolve`]: high-level orchestrator turning SQL into validated metadata.
//! - [`introspect`]: online-only helpers for talking to a live PostgreSQL.
//! - [`error`]: typed errors.

pub mod cache;
pub mod config;
pub mod error;
pub mod normalize;
pub mod nullable;
pub mod parser;
pub mod resolve;
pub mod schema;
pub mod scope;

#[cfg(feature = "online")]
pub mod introspect;

pub use error::{Error, Result};

