//! Test-harness runtime for `#[sntl::test]`.
//!
//! This module is enabled unconditionally — it has no proc-macro
//! dependencies and only adds three small files. The attribute macro
//! that drives it lives in `sntl-macros`.
//!
//! See `docs/testing-guide.md` for the full user guide.

pub mod fixtures;
pub mod run;
pub mod template;

pub use run::run;
