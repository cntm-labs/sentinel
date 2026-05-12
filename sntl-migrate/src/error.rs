use std::path::PathBuf;
use thiserror::Error;

use crate::migration::Version;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid migration folder name `{name}`: expected YYYYMMDD_HHMMSS_<snake_case>")]
    InvalidName { name: String },

    #[error("migration `{pending}` has timestamp before highest applied `{highest_applied}`")]
    OutOfOrder {
        pending: Version,
        highest_applied: Version,
    },

    #[error("migration `{version}` failed: {source}")]
    ApplyFailed {
        version: Version,
        #[source]
        source: sentinel_driver::Error,
    },

    #[error("checksum mismatch for applied migration `{version}` — file modified after apply")]
    ChecksumDrift {
        version: Version,
        file: String,
        recorded: String,
    },

    #[error("could not acquire migration lock — another process is migrating")]
    LockBusy,

    #[error("driver error: {0}")]
    Driver(#[from] sentinel_driver::Error),

    #[error("schema introspection failed: {0}")]
    Introspect(#[from] sntl_schema::Error),

    #[error("migrations directory missing or unreadable: {path}")]
    MissingDir { path: PathBuf },
}
