/// Sentinel error types.
///
/// All errors are `Send + Sync` so they work across async boundaries.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("column '{column}' not found in table '{table}'")]
    ColumnNotFound { column: String, table: String },

    #[error("query build error: {0}")]
    QueryBuild(String),

    #[error("connection error: {0}")]
    Connection(String),

    #[error("transaction error: {0}")]
    Transaction(String),

    #[error("row not found")]
    NotFound,

    #[error("type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },

    #[error("driver error: {0}")]
    Driver(#[from] sentinel_driver::Error),
}

/// Sentinel result type alias.
pub type Result<T> = std::result::Result<T, Error>;
