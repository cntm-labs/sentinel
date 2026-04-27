//! `sntl prepare` — scan workspace, pull schema, write `.sentinel/` cache.

use crate::{scan, ui};
use anyhow::{Context, Result, anyhow};
use sntl_schema::cache::{Cache, SourceLocation};
use sntl_schema::config::Config;
use std::path::PathBuf;

pub async fn run(
    workspace: Option<PathBuf>,
    database_url: Option<String>,
    check_only: bool,
) -> Result<()> {
    let root =
        workspace.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let cfg_path = root.join("sentinel.toml");
    let mut cfg = Config::load_from(&cfg_path)?;
    if let Some(url) = database_url {
        cfg.database.url = Some(url);
    }
    let url = cfg.database.url.clone().ok_or_else(|| {
        anyhow!(
            "no database_url — set SENTINEL_DATABASE_URL, pass --database-url, or add [database] to sentinel.toml"
        )
    })?;

    let cache = Cache::new(root.join(cfg.cache_dir()));
    cache.init().context("init .sentinel/")?;
    cache.check_version().context("check cache version")?;

    ui::ok("scanning workspace");
    let found = scan::scan(&root)?;
    let mut queries: std::collections::BTreeMap<String, (String, Vec<SourceLocation>)> =
        std::collections::BTreeMap::new();
    for d in found {
        let normalized = sntl_schema::normalize::normalize_sql(&d.sql);
        let hash = sntl_schema::normalize::hash_sql(&d.sql);
        queries
            .entry(hash.clone())
            .or_insert_with(|| (normalized.clone(), vec![]))
            .1
            .push(SourceLocation {
                file: d.file.to_string_lossy().into(),
                line: d.line,
            });
    }

    if queries.is_empty() {
        ui::warn("no sntl::query!() invocations found — nothing to prepare");
        return Ok(());
    }

    ui::ok(&format!("found {} distinct queries", queries.len()));

    let schema = sntl_schema::introspect::pull_schema(&url)
        .await
        .map_err(|e| anyhow!("pull schema: {e}"))?;
    if !check_only {
        cache.write_schema(&schema).context("write schema.toml")?;
    }

    let pb = ui::progress(queries.len() as u64, "preparing queries");
    let mut stale = 0u32;
    for (hash, (sql, locs)) in queries {
        let entry = sntl_schema::introspect::prepare_query(&url, &sql, locs)
            .await
            .map_err(|e| anyhow!("prepare {sql:?}: {e}"))?;
        pb.inc(1);
        if check_only {
            match cache.read_entry(&hash) {
                Ok(existing) if existing.sql_normalized == entry.sql_normalized => {}
                _ => stale += 1,
            }
        } else {
            cache.write_entry(&entry)?;
        }
    }
    pb.finish_and_clear();

    if check_only && stale > 0 {
        ui::err(&format!("{stale} queries are stale — run `sntl prepare`"));
        std::process::exit(1);
    }
    ui::ok("all queries cached");
    Ok(())
}
