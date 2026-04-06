//! Common imports for Sentinel users.
//!
//! ```rust
//! use sntl::prelude::*;
//! ```

pub use crate::core::error::{Error, Result};
pub use crate::core::expr::{Column, Expr, OrderExpr};
pub use crate::core::model::{Model, ModelColumn};
pub use crate::core::query::{DeleteQuery, InsertQuery, QueryBuilder, SelectQuery, UpdateQuery};
pub use crate::core::types::Value;

// Re-export derive macros
pub use macros::Model as DeriveModel;
pub use macros::Partial as DerivePartial;

// Re-export driver types
pub use driver::{Config, Connection, Pool};
