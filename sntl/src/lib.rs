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
//!
//! ## Day-one DX (v0.5)
//!
//! - **`Pool` is a first-class macro argument.** Pass `&pool` directly to
//!   `fetch_*` / `execute` instead of pulling a connection by hand:
//!   ```ignore
//!   let users: Vec<User> = sntl::query_as!(User, "SELECT * FROM users")
//!       .fetch_all(&pool).await?;
//!   ```
//!   See [`__macro_support::QueryExecution`].
//!
//! - **Streaming.** `fetch_stream(&mut conn)` returns a [`RowStream`] that
//!   yields rows lazily without materialising the whole result set.
//!
//! - **`#[sntl::test]`.** Fixture-isolated test harness — every test gets
//!   a fresh PostgreSQL database via `CREATE DATABASE ... TEMPLATE`. See
//!   [`testing`] and `docs/testing-guide.md`.
//!
//! ## Guides
//!
//! - `docs/migration-from-sqlx.md` — switch from `sqlx::query!` to `sntl::query!`
//! - `docs/migration-guide.md` — write and run schema migrations with `sntl-migrate`
//! - `docs/observability-guide.md` — wire `tracing` / OpenTelemetry to every query
//! - `docs/testing-guide.md` — write fixture-isolated tests with `#[sntl::test]`

#[doc(hidden)]
pub mod __priv;

/// Core — Model trait, QueryBuilder, types, and connection abstraction.
pub mod core;

/// Migration tools — schema diff and migration generation.
pub mod migrate;

/// Test-harness runtime for `#[sntl::test]` — fixture-isolated per-test databases.
pub mod testing;

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
pub use macros::{
    query, query_as, query_as_unchecked, query_file, query_file_as, query_pipeline, query_scalar,
    query_unchecked,
};

/// Attribute macro — `#[sentinel(relations)]`.
pub use macros::sentinel;

/// Attribute macro — `#[sntl::reducer]` wraps an async fn in BEGIN/COMMIT/ROLLBACK with auto-rollback on Err/panic.
pub use macros::reducer;

/// Attribute macro — `#[sntl::test]` for fixture-isolated integration tests.
pub use macros::test;

pub use core::error::{Error, Result};
pub use core::observability;

// Re-export driver traits for custom PG types
pub use driver::{FromSql, ToSql};

// Re-export key driver types for direct use
pub use driver::{Config, Oid, Pool, PooledConnection, RowStream};

/// PostgreSQL extension types re-exported from the driver.
pub mod types {
    pub use driver::types::{cube, hstore, ltree};
}

/// Internal API used by `sntl::query!` family macros. Not covered by semver.
#[doc(hidden)]
pub mod __macro_support {
    pub use crate::core::query::macro_support::*;
    pub use ::futures::FutureExt;
}
