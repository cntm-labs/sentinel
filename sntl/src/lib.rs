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

/// Derive macros — `#[derive(Model)]`, `#[derive(Partial)]`.
pub use macros::Model;
pub use macros::Partial;

/// Attribute macro — `#[sentinel(relations)]`.
pub use macros::sentinel;

pub use core::error::{Error, Result};
