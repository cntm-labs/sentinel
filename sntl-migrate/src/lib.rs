//! Forward-only SQL migrations for Sentinel ORM.
//!
//! Two entry points:
//! - `Migrator::from_dir("migrations/")` for the CLI and dev workflows.
//! - `sntl_migrate::migrate!("./migrations")` (re-exported from `sntl-macros`)
//!   for compile-time embedding into a production binary.
//!
//! See `docs/migration-guide.md` for the full user guide.

pub mod checksum;
pub mod diff;
pub mod discover;
pub mod error;
pub mod macro_support;
pub mod migration;
pub mod refresh;
pub mod runner;
pub mod tracking;

pub use error::{Error, Result};
pub use migration::{Migration, TxMode, Version};
pub use runner::{MigrationReport, MigrationStatus, Migrator, RefreshConfig, State};
pub use sntl_macros::migrate;

/// The PostgreSQL advisory-lock ID used to serialise concurrent migrators.
/// ASCII bytes "sntlmgrt" — chosen to be unlikely to collide with other tools.
pub const SNTL_MIGRATE_LOCK_ID: i64 = 0x736e_746c_6d67_7274_i64;
