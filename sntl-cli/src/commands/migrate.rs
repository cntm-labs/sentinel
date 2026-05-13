//! `sntl migrate` — scaffold, apply, inspect, diff, and verify migrations.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use sentinel_driver::pool::config::PoolConfig;
use sntl_migrate::{Migrator, State};
use sntl_schema::config::Config;

use crate::MigrateCmd;
use crate::ui;

/// Route a parsed `MigrateCmd` to the right handler.
pub async fn dispatch(
    workspace: Option<PathBuf>,
    database_url: Option<String>,
    action: MigrateCmd,
) -> Result<()> {
    match action {
        MigrateCmd::Add {
            name,
            no_create_dir,
        } => add(workspace, name, no_create_dir).await,
        MigrateCmd::Run {
            dry_run,
            skip_refresh,
        } => run(workspace, database_url, dry_run, skip_refresh).await,
        MigrateCmd::Info {
            applied,
            pending,
            all,
        } => info(workspace, database_url, applied, pending, all).await,
        MigrateCmd::Diff { out } => diff(workspace, database_url, out).await,
        MigrateCmd::Verify => verify(workspace, database_url).await,
    }
}

/// `sntl migrate add <name>` — create `migrations/<timestamp>_<sanitised>/up.sql`.
pub async fn add(workspace: Option<PathBuf>, name: String, no_create_dir: bool) -> Result<()> {
    let root = workspace
        .or_else(|| std::env::current_dir().ok())
        .context("cannot resolve workspace root")?;
    let migrations = root.join("migrations");
    if !migrations.exists() {
        if no_create_dir {
            return Err(anyhow!(
                "migrations/ does not exist (drop --no-create-dir to auto-create it)"
            ));
        }
        std::fs::create_dir_all(&migrations).context("create migrations/")?;
    }
    let sanitised = sanitise_name(&name)?;
    let ts = utc_now_compact();
    let folder = migrations.join(format!("{ts}_{sanitised}"));
    if folder.exists() {
        return Err(anyhow!("{} already exists", folder.display()));
    }
    std::fs::create_dir_all(&folder).context("create migration folder")?;
    let up = folder.join("up.sql");
    std::fs::write(&up, header_template(&ts, &sanitised)).context("write up.sql")?;
    ui::ok(&format!("created {}", up.display()));
    println!("ℹ edit it, then run `sntl migrate run`");
    Ok(())
}

/// `sntl migrate run` — apply pending migrations, optionally refresh schema.toml.
pub async fn run(
    workspace: Option<PathBuf>,
    database_url: Option<String>,
    dry_run: bool,
    skip_refresh: bool,
) -> Result<()> {
    let (root, url) = resolve(workspace, database_url)?;
    let migrations_dir = root.join("migrations");
    let migrator = Migrator::from_dir(&migrations_dir)
        .with_context(|| format!("discover {}", migrations_dir.display()))?;

    if dry_run {
        ui::ok("dry-run — would apply:");
        for m in migrator.migrations() {
            println!("  ◯ {}", m.version);
        }
        return Ok(());
    }

    let pool = pool_for(&url)?;
    let migrator = if skip_refresh {
        migrator
    } else {
        migrator.with_refresh(url.clone(), root.join(".sentinel"))
    };

    ui::ok("acquired migration lock");
    let report = migrator.run(&pool).await.context("apply migrations")?;
    if report.applied.is_empty() {
        ui::ok("no pending migrations");
    } else {
        for v in &report.applied {
            ui::ok(&format!("applied {v}"));
        }
        ui::ok(&format!("{} migration(s) applied", report.applied.len()));
    }
    if !skip_refresh && !report.applied.is_empty() {
        ui::ok("refreshed .sentinel/schema.toml");
    }
    Ok(())
}

/// `sntl migrate info` — show applied + pending state.
pub async fn info(
    workspace: Option<PathBuf>,
    database_url: Option<String>,
    show_applied: bool,
    show_pending: bool,
    show_all: bool,
) -> Result<()> {
    let (root, url) = resolve(workspace, database_url)?;
    let migrations_dir = root.join("migrations");
    let migrator = Migrator::from_dir(&migrations_dir)?;
    let pool = pool_for(&url)?;
    let statuses = migrator.info(&pool).await?;

    let no_filter = !(show_applied || show_pending || show_all);
    let want_applied = show_applied || show_all || no_filter;
    let want_pending = show_pending || show_all || no_filter;

    for s in &statuses {
        let label = match s.state {
            State::Applied if want_applied => "✓",
            State::Pending if want_pending => "◯",
            State::ChecksumDrift if want_applied => "⚠",
            _ => continue,
        };
        let cs = s.checksum.as_deref().unwrap_or("");
        println!("  {label} {}  {cs}", s.version);
    }
    Ok(())
}

/// `sntl migrate verify` — error out if any applied migration has drifted.
pub async fn verify(workspace: Option<PathBuf>, database_url: Option<String>) -> Result<()> {
    let (root, url) = resolve(workspace, database_url)?;
    let migrations_dir = root.join("migrations");
    let migrator = Migrator::from_dir(&migrations_dir)?;
    let pool = pool_for(&url)?;
    let statuses = migrator.info(&pool).await?;
    let drifted: Vec<_> = statuses
        .iter()
        .filter(|s| s.state == State::ChecksumDrift)
        .collect();
    if drifted.is_empty() {
        let applied = statuses
            .iter()
            .filter(|s| s.state == State::Applied)
            .count();
        ui::ok(&format!(
            "all {applied} applied migration(s) have matching checksums"
        ));
        Ok(())
    } else {
        for d in &drifted {
            ui::warn(&format!("checksum drift in {}", d.version));
        }
        Err(anyhow!("verify failed"))
    }
}

/// `sntl migrate diff` — compare cache vs live DB, write SQL scaffold.
pub async fn diff(
    workspace: Option<PathBuf>,
    database_url: Option<String>,
    out: Option<String>,
) -> Result<()> {
    let (root, url) = resolve(workspace, database_url)?;
    let cache_path = root.join(".sentinel/schema.toml");
    let cache_text = std::fs::read_to_string(&cache_path)
        .with_context(|| format!("read {}", cache_path.display()))?;
    let cache: sntl_schema::schema::Schema =
        toml::from_str(&cache_text).context("parse .sentinel/schema.toml")?;
    let live = sntl_schema::introspect::pull_schema(&url)
        .await
        .context("pull live schema")?;

    let changes = sntl_migrate::diff::compare(&cache, &live);
    if changes.is_empty() {
        ui::ok("no differences");
        return Ok(());
    }
    let (sql, todos) = sntl_migrate::diff::emit(&changes);

    let ts = utc_now_compact();
    let suffix = out.unwrap_or_else(|| "diff".to_string());
    let folder = root.join("migrations").join(format!("{ts}_{suffix}"));
    std::fs::create_dir_all(&folder).context("create migration folder")?;
    let up = folder.join("up.sql");
    std::fs::write(&up, sql).context("write up.sql")?;

    ui::ok(&format!("wrote {}", up.display()));
    println!("ℹ {} change(s) ({} TODO)", changes.len(), todos);
    Ok(())
}

fn sanitise_name(name: &str) -> Result<String> {
    let mut out = String::with_capacity(name.len());
    let mut last_underscore = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_underscore = false;
        } else if !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        return Err(anyhow!("migration name empty after sanitisation"));
    }
    Ok(trimmed)
}

fn utc_now_compact() -> String {
    chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()
}

fn header_template(ts: &str, name: &str) -> String {
    format!(
        "-- Migration: {ts}_{name}\n\
         -- Created: {ts} UTC\n\
         --\n\
         -- This file runs in a single PostgreSQL transaction. Rename to\n\
         -- `up.notx.sql` if you need non-transactional DDL (CREATE INDEX\n\
         -- CONCURRENTLY, REFRESH MATERIALIZED VIEW CONCURRENTLY, etc.).\n\
         \n"
    )
}

fn resolve(workspace: Option<PathBuf>, database_url: Option<String>) -> Result<(PathBuf, String)> {
    let root = workspace
        .or_else(|| std::env::current_dir().ok())
        .context("cannot resolve workspace root")?;
    let mut cfg = Config::load_from(root.join("sentinel.toml")).unwrap_or_default();
    if let Some(u) = database_url {
        cfg.database.url = Some(u);
    }
    let url = cfg.database.url.ok_or_else(|| {
        anyhow!(
            "no database_url — set SENTINEL_DATABASE_URL, pass --database-url, or add [database] to sentinel.toml"
        )
    })?;
    Ok((root, url))
}

fn pool_for(url: &str) -> Result<sentinel_driver::Pool> {
    let cfg = sentinel_driver::Config::parse(url).context("parse DATABASE_URL")?;
    Ok(sentinel_driver::Pool::new(
        cfg,
        PoolConfig::new().max_connections(4),
    ))
}
