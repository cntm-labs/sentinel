//! `sntl check` — validate `.sentinel/` cache against current source.

use crate::{scan, ui};
use anyhow::{Context, Result};
use sntl_schema::cache::Cache;
use sntl_schema::config::Config;
use std::path::PathBuf;

pub async fn run(workspace: Option<PathBuf>) -> Result<()> {
    let root =
        workspace.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let cfg = Config::load_from(root.join("sentinel.toml"))?;
    let cache = Cache::new(root.join(cfg.cache_dir()));
    cache.check_version().context("cache version")?;

    let entries = cache.list_entries()?;
    let found = scan::scan(&root)?;
    let mut referenced = std::collections::HashSet::new();
    for d in &found {
        referenced.insert(sntl_schema::normalize::hash_sql(&d.sql));
    }

    let mut orphaned = 0u32;
    for e in &entries {
        if !referenced.contains(&e.sql_hash) {
            ui::warn(&format!(
                "orphaned cache entry: {} (no source reference)",
                e.sql_hash
            ));
            orphaned += 1;
        }
    }

    let mut missing = 0u32;
    for d in &found {
        let h = sntl_schema::normalize::hash_sql(&d.sql);
        if cache.read_entry(&h).is_err() {
            ui::err(&format!(
                "missing cache for {}:{}",
                d.file.display(),
                d.line
            ));
            missing += 1;
        }
    }

    if missing > 0 {
        ui::err(&format!(
            "{missing} queries not in cache — run `sntl prepare`"
        ));
        std::process::exit(1);
    }
    if orphaned > 0 {
        ui::warn(&format!(
            "{orphaned} orphaned entries — run `sntl prepare` to rebuild"
        ));
    }
    ui::ok("cache is consistent");
    Ok(())
}
