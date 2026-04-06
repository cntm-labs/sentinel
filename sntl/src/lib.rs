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
pub mod core {
    pub use sntl_core::*;
}

/// Derive macros — `#[derive(Model)]`, `#[derive(Partial)]`.
pub mod macros {
    pub use sntl_macros::Model;
    pub use sntl_macros::Partial;
}

/// Migration tools — schema diff and migration generation.
pub mod migrate {
    pub use sntl_migrate::*;
}

/// Prelude — common imports for quick setup.
///
/// ```rust
/// use sntl::prelude::*;
/// ```
pub mod prelude {
    pub use sntl_core::prelude::*;
}
