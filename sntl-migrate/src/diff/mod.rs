//! Schema-diff scaffolding: compute structural changes between two
//! `sntl_schema::Schema` snapshots and emit a SQL skeleton for review.

pub mod compare;
pub mod emit;

pub use compare::{Change, compare};
pub use emit::emit;
