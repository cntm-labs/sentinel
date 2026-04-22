//! `sntl doctor` — diagnostic checklist for config, DB, and cache health.

use crate::ui;
use anyhow::Result;
use sntl_schema::cache::Cache;
use sntl_schema::config::Config;
use std::path::PathBuf;

pub async fn run(workspace: Option<PathBuf>, database_url: Option<String>) -> Result<()> {
    let root =
        workspace.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let cfg_path = root.join("sentinel.toml");
    if cfg_path.exists() {
        ui::ok(&format!("sentinel.toml found at {}", cfg_path.display()));
    } else {
        ui::warn("sentinel.toml missing — create one (see docs)");
    }

    let mut cfg = Config::load_from(&cfg_path)?;
    if let Some(u) = database_url {
        cfg.database.url = Some(u);
    }

    match &cfg.database.url {
        Some(url) => match sntl_schema::introspect::pull_schema(url).await {
            Ok(s) => ui::ok(&format!(
                "database connection OK (PostgreSQL {})",
                s.postgres_version
            )),
            Err(e) => ui::err(&format!("cannot reach database: {e}")),
        },
        None => ui::err("no database_url configured"),
    }

    let cache = Cache::new(root.join(cfg.cache_dir()));
    match cache.check_version() {
        Ok(()) => ui::ok(".sentinel/ cache version compatible"),
        Err(e) => ui::err(&format!(".sentinel/ cache problem: {e}")),
    }

    match cache.list_entries() {
        Ok(v) => ui::ok(&format!("{} query entries in cache", v.len())),
        Err(e) => ui::err(&format!("cache unreadable: {e}")),
    }

    Ok(())
}
