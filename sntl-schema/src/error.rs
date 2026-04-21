use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("TOML parse error in {path}: {source}")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("JSON parse error in {path}: {source}")]
    JsonParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("SQL parse error: {0}")]
    SqlParse(String),

    #[error("cache format version {found} is newer than supported {supported}; upgrade sntl-macros")]
    CacheVersionTooNew { found: u32, supported: u32 },

    #[error("cache miss: query not found at {path}")]
    CacheMiss { path: PathBuf },

    #[error("schema snapshot missing table `{table}`")]
    UnknownTable { table: String },

    #[error("schema snapshot missing column `{table}.{column}`")]
    UnknownColumn { table: String, column: String },

    #[error("column ambiguity: `{column}` could refer to multiple tables: {candidates:?}")]
    AmbiguousColumn {
        column: String,
        candidates: Vec<String>,
    },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("introspection error: {0}")]
    Introspect(String),
}
