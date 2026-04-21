use crate::error::{Error, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub offline: OfflineConfig,
    pub schema: SchemaConfig,
    pub macros: MacrosConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfflineMode {
    On,
    Off,
}

impl Default for OfflineMode {
    fn default() -> Self {
        OfflineMode::Off
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct OfflineConfig {
    #[serde(deserialize_with = "deserialize_offline_flag")]
    pub enabled: OfflineMode,
}

fn deserialize_offline_flag<'de, D>(d: D) -> std::result::Result<OfflineMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: toml::Value = toml::Value::deserialize(d)?;
    Ok(match v {
        toml::Value::Boolean(true) => OfflineMode::On,
        toml::Value::Boolean(false) => OfflineMode::Off,
        toml::Value::String(s) if s == "on" || s == "true" => OfflineMode::On,
        _ => OfflineMode::Off,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchemaConfig {
    pub path: String,
    pub dialect: String,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            path: ".sentinel/schema.toml".into(),
            dialect: "postgres-16".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MacrosConfig {
    pub strict_nullable: bool,
    pub deny_warnings: bool,
}

impl Default for MacrosConfig {
    fn default() -> Self {
        Self {
            strict_nullable: true,
            deny_warnings: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    pub dir: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: ".sentinel".into(),
        }
    }
}

impl Config {
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut cfg: Config = match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|source| Error::TomlParse {
                path: path.to_path_buf(),
                source,
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(source) => {
                return Err(Error::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        cfg.apply_env();
        Ok(cfg)
    }

    fn apply_env(&mut self) {
        if let Ok(url) = std::env::var("SENTINEL_DATABASE_URL") {
            self.database.url = Some(url);
        }
        match std::env::var("SENTINEL_OFFLINE").as_deref() {
            Ok("true") | Ok("1") | Ok("on") => self.offline.enabled = OfflineMode::On,
            Ok("false") | Ok("0") | Ok("off") => self.offline.enabled = OfflineMode::Off,
            _ => {}
        }
        if let Ok(dir) = std::env::var("SENTINEL_CACHE_DIR") {
            self.cache.dir = dir;
        }
    }

    pub fn cache_dir(&self) -> PathBuf {
        PathBuf::from(&self.cache.dir)
    }
}
