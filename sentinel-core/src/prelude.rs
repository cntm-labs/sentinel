//! Common imports for Sentinel users.
//!
//! ```rust
//! use sentinel_core::prelude::*;
//! ```

pub use crate::error::{Error, Result};
pub use crate::expr::{Column, Expr, OrderExpr};
pub use crate::model::{Model, ModelColumn};
pub use crate::query::{DeleteQuery, InsertQuery, QueryBuilder, SelectQuery, UpdateQuery};
pub use crate::types::Value;

// Re-export derive macros
pub use sentinel_macros::Model as DeriveModel;
pub use sentinel_macros::Partial as DerivePartial;
