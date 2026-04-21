use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CACHE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub version: u32,
    pub sql_hash: String,
    pub sql_normalized: String,
    #[serde(default)]
    pub source_locations: Vec<SourceLocation>,
    pub params: Vec<ParamInfo>,
    pub columns: Vec<ColumnInfo>,
    pub query_kind: QueryKind,
    #[serde(default)]
    pub has_returning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamInfo {
    pub index: u32,
    pub pg_type: String,
    pub oid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub pg_type: String,
    pub oid: u32,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub origin: Option<ColumnOrigin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnOrigin {
    pub table: String,
    pub column: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryKind {
    Select,
    Insert,
    Update,
    Delete,
    Other,
}

pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    pub fn init(&self) -> Result<()> {
        let queries = self.dir.join("queries");
        std::fs::create_dir_all(&queries).map_err(|source| Error::Io {
            path: queries.clone(),
            source,
        })?;
        let version_file = self.dir.join(".version");
        if !version_file.exists() {
            std::fs::write(&version_file, CACHE_FORMAT_VERSION.to_string()).map_err(|source| {
                Error::Io {
                    path: version_file,
                    source,
                }
            })?;
        }
        Ok(())
    }

    pub fn read_version(&self) -> Result<u32> {
        let p = self.dir.join(".version");
        let text = std::fs::read_to_string(&p).map_err(|source| Error::Io {
            path: p.clone(),
            source,
        })?;
        text.trim()
            .parse()
            .map_err(|_| Error::Config(format!("invalid cache version: {text:?}")))
    }

    pub fn check_version(&self) -> Result<()> {
        let found = self.read_version()?;
        if found > CACHE_FORMAT_VERSION {
            return Err(Error::CacheVersionTooNew {
                found,
                supported: CACHE_FORMAT_VERSION,
            });
        }
        Ok(())
    }

    pub fn query_path(&self, hash: &str) -> PathBuf {
        self.dir.join("queries").join(format!("{hash}.json"))
    }

    pub fn read_entry(&self, hash: &str) -> Result<CacheEntry> {
        let path = self.query_path(hash);
        if !path.exists() {
            return Err(Error::CacheMiss { path });
        }
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| Error::JsonParse { path, source })
    }

    pub fn write_entry(&self, entry: &CacheEntry) -> Result<()> {
        let path = self.query_path(&entry.sql_hash);
        let text = serde_json::to_string_pretty(entry).map_err(|source| Error::JsonParse {
            path: path.clone(),
            source,
        })?;
        std::fs::write(&path, text).map_err(|source| Error::Io { path, source })?;
        Ok(())
    }

    pub fn schema_path(&self) -> PathBuf {
        self.dir.join("schema.toml")
    }

    pub fn read_schema(&self) -> Result<crate::schema::Schema> {
        let p = self.schema_path();
        let text = std::fs::read_to_string(&p).map_err(|source| Error::Io {
            path: p.clone(),
            source,
        })?;
        toml::from_str(&text).map_err(|source| Error::TomlParse { path: p, source })
    }

    pub fn write_schema(&self, schema: &crate::schema::Schema) -> Result<()> {
        let p = self.schema_path();
        let text = toml::to_string_pretty(schema)
            .map_err(|e| Error::Config(format!("schema serialize: {e}")))?;
        std::fs::write(&p, text).map_err(|source| Error::Io { path: p, source })?;
        Ok(())
    }

    pub fn list_entries(&self) -> Result<Vec<CacheEntry>> {
        let queries = self.dir.join("queries");
        let mut out = vec![];
        let rd = match std::fs::read_dir(&queries) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(source) => {
                return Err(Error::Io {
                    path: queries,
                    source,
                });
            }
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "json") {
                let text = std::fs::read_to_string(&p).map_err(|source| Error::Io {
                    path: p.clone(),
                    source,
                })?;
                let ce: CacheEntry =
                    serde_json::from_str(&text).map_err(|source| Error::JsonParse {
                        path: p.clone(),
                        source,
                    })?;
                out.push(ce);
            }
        }
        Ok(out)
    }
}
