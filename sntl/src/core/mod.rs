//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod model;
pub mod prelude;
pub mod query;
pub mod relation;
pub mod transaction;
pub mod types;

// Flat re-exports — clean public API
pub use error::{Error, Result};
pub use expr::{Column, Expr, OrderExpr};
pub use model::{Model, ModelColumn};
pub use query::{DeleteQuery, InsertQuery, ModelQuery, QueryBuilder, SelectQuery, UpdateQuery};
pub use relation::{RelationStore, WithRelations};
pub use transaction::Transaction;
pub use types::Value;

// Re-export derive macros
pub use macros::Model as DeriveModel;
pub use macros::Partial as DerivePartial;

// Re-export driver types
pub use driver::Row;
pub use driver::RowStream;
pub use driver::ToSql;
pub use driver::{
    CancelToken, Config, Connection, IsolationLevel, Pool, PooledConnection, SslMode,
    TransactionConfig,
};
