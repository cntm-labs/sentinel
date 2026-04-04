//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod model;
pub mod prelude;
pub mod query;
pub mod types;

pub use error::{Error, Result};

// Re-export derive macros so users write `use sentinel_core::Model;`
pub use sentinel_macros::Model;
