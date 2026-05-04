//! `sntl init` — scaffold `sentinel.toml` and `.sentinel/` in the workspace root.

use crate::ui;
use anyhow::{Context, Result};
use sntl_schema::cache::Cache;
use std::path::{Path, PathBuf};

/// Default `sentinel.toml` template. The `database.url` line is commented out
/// so projects don't accidentally commit credentials — set
/// `SENTINEL_DATABASE_URL` or pass `--database-url` to override.
const SENTINEL_TOML_TEMPLATE: &str = r#"# Sentinel ORM configuration.
#
# Override at runtime with the SENTINEL_* environment variables, e.g.
# SENTINEL_DATABASE_URL, SENTINEL_OFFLINE, SENTINEL_CACHE_DIR.

[database]
# url = "postgres://user:password@localhost:5432/dbname"

[macros]
# Treat unknown column nullability as Option<T> (recommended).
strict_nullable = true
deny_warnings = false

[cache]
# Where the committed `.sentinel/` cache lives, relative to the workspace root.
dir = ".sentinel"
"#;

/// Default `.sentinel/.gitignore`. Cache files (queries/, schema.toml,
/// .version) MUST be committed — they're the offline source of truth that
/// the macros consult at compile time. Only ignore working directories the
/// CLI may scribble in.
const SENTINEL_GITIGNORE: &str =
    "# Trybuild + cargo-llvm-cov work directories the CLI never produces but
# users sometimes drop here. Everything inside the cache directory itself
# stays tracked — those files ARE the offline cache the macros consult at
# compile time.
/wip/
/tmp/
";

pub fn run(workspace: Option<PathBuf>, force: bool) -> Result<()> {
    let root = workspace
        .or_else(|| std::env::current_dir().ok())
        .context("cannot resolve workspace root")?;

    let toml_path = root.join("sentinel.toml");
    let cache_dir = root.join(".sentinel");
    let gitignore_path = cache_dir.join(".gitignore");

    write_sentinel_toml(&toml_path, force)?;
    init_cache_dir(&cache_dir)?;
    write_gitignore(&gitignore_path, force)?;

    ui::ok(&format!("initialised {}", root.display()));
    println!();
    println!("Next steps:");
    println!("  1. Edit sentinel.toml or set SENTINEL_DATABASE_URL");
    println!("  2. Run `sntl prepare` to populate .sentinel/queries/");
    println!("  3. Commit `.sentinel/` so CI builds work without a DB");
    Ok(())
}

fn write_sentinel_toml(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        ui::warn(&format!(
            "sentinel.toml already exists at {} — left untouched (use --force to overwrite)",
            path.display()
        ));
        return Ok(());
    }
    std::fs::write(path, SENTINEL_TOML_TEMPLATE)
        .with_context(|| format!("write {}", path.display()))?;
    ui::ok(&format!("wrote {}", path.display()));
    Ok(())
}

fn init_cache_dir(dir: &Path) -> Result<()> {
    let cache = Cache::new(dir);
    cache
        .init()
        .with_context(|| format!("init {}", dir.display()))?;
    ui::ok(&format!("initialised cache at {}", dir.display()));
    Ok(())
}

fn write_gitignore(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        // Don't warn here — many users symlink or pre-create .sentinel/.gitignore
        // and we don't want to nag on every re-run.
        return Ok(());
    }
    std::fs::write(path, SENTINEL_GITIGNORE)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
