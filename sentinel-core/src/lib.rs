//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod model;
pub mod prelude;
pub mod query;
pub mod types;

pub use error::{Error, Result};

// Re-export derive macros
pub use sentinel_macros::Model;
pub use sentinel_macros::Partial;

// Re-export driver types for user convenience
pub use sentinel_driver::Row;
pub use sentinel_driver::{CancelToken, IsolationLevel, TransactionConfig};
pub use sentinel_driver::{Config, Connection, Pool, SslMode};
