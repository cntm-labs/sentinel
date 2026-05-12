use std::path::Path;

use crate::error::{Error, Result};
use crate::migration::{Migration, TxMode, Version};

/// Walk the `migrations/` directory, parse each folder as a `Version`, and
/// return migrations in ascending version order.
pub fn discover(migrations_dir: &Path) -> Result<Vec<Migration>> {
    if !migrations_dir.exists() || !migrations_dir.is_dir() {
        return Err(Error::MissingDir {
            path: migrations_dir.to_path_buf(),
        });
    }

    let mut out: Vec<Migration> = Vec::new();
    let rd = std::fs::read_dir(migrations_dir).map_err(|source| Error::Io {
        path: migrations_dir.to_path_buf(),
        source,
    })?;

    for entry in rd {
        let entry = entry.map_err(|source| Error::Io {
            path: migrations_dir.to_path_buf(),
            source,
        })?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let version: Version = name.parse()?;
        let (path, tx_mode) = pick_sql_file(&entry.path())?;
        let sql = std::fs::read_to_string(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        out.push(Migration {
            version,
            sql,
            tx_mode,
        });
    }

    out.sort_by(|a, b| a.version.cmp(&b.version));
    Ok(out)
}

fn pick_sql_file(dir: &Path) -> Result<(std::path::PathBuf, TxMode)> {
    let notx = dir.join("up.notx.sql");
    if notx.exists() {
        return Ok((notx, TxMode::None));
    }
    let up = dir.join("up.sql");
    if up.exists() {
        return Ok((up, TxMode::PerMigration));
    }
    Err(Error::Io {
        path: dir.to_path_buf(),
        source: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "neither up.sql nor up.notx.sql found",
        ),
    })
}
