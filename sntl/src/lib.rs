//! # Sentinel ORM
//!
//! Compile-time guarded ORM for PostgreSQL.
//!
//! ```toml
//! [dependencies]
//! sntl = "0.1"
//! ```
//!
//! ```rust
//! use sntl::prelude::*;
//! ```

/// Core — Model trait, QueryBuilder, types, and connection abstraction.
pub mod core;

/// Migration tools — schema diff and migration generation.
pub mod migrate;

/// Prelude — common imports for quick setup.
pub mod prelude {
    pub use crate::core::prelude::*;
}

/// PostgreSQL wire protocol driver.
pub use driver;

/// Derive macros — `#[derive(Model)]`, `#[derive(Partial)]`, `#[derive(FromRow)]`.
pub use macros::FromRow;
pub use macros::Model;
pub use macros::Partial;

/// Compile-time-validated `query!()` family.
pub use macros::query;

/// Attribute macro — `#[sentinel(relations)]`.
pub use macros::sentinel;

pub use core::error::{Error, Result};

// Re-export driver traits for custom PG types
pub use driver::{FromSql, ToSql};

// Re-export key driver types for direct use
pub use driver::{Config, Oid, Pool, PooledConnection};
pub use driver::{ObservabilityConfig, QueryMetrics};

/// Internal API used by `sntl::query!` family macros. Not covered by semver.
#[doc(hidden)]
pub mod __macro_support {
    pub use crate::core::query::macro_support::*;
}
