//! Template-database lifecycle. Builds `_sntl_tmpl_<hash>` once per
//! process per migrations directory; clones it per test.
//!
//! ## Security note on DDL interpolation
//!
//! `CREATE DATABASE`, `DROP DATABASE`, and related DDL statements cannot be
//! parameterised in PostgreSQL. Every database name used here is either:
//! - Derived from a SHA-256 hex digest (template names), or
//! - Prefixed with `_sntl_` and sourced from a sanitised test suffix
//!   supplied by `run::run` (clone names).
//!
//! No user-supplied data reaches these strings without going through the
//! digest or the sanitiser in `run.rs`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use driver::pool::config::PoolConfig;
use driver::{Config, Connection, Pool};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

/// Metadata for a built template database.
#[derive(Debug)]
pub struct TemplateDb {
    pub db_name: String,
    pub admin_url: String,
}

/// Build (once per process) or look up the template DB for the given
/// migrations directory, then return its metadata.
///
/// The cache key is a 4-byte SHA-256 prefix of the canonical migrations
/// path (or "none" when no migrations are provided). Two callers with the
/// same path share one template DB.
pub async fn build_or_get(
    admin_url: &str,
    migrations_dir: Option<&Path>,
) -> anyhow::Result<TemplateDb> {
    static CACHE: OnceLock<Mutex<HashMap<String, TemplateDb>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let key = template_key(migrations_dir)?;
    let db_name = format!("_sntl_tmpl_{key}");

    let mut guard = cache.lock().await;
    if let Some(existing) = guard.get(&key) {
        return Ok(TemplateDb {
            db_name: existing.db_name.clone(),
            admin_url: existing.admin_url.clone(),
        });
    }

    create_template(admin_url, &db_name, migrations_dir).await?;
    let entry = TemplateDb {
        db_name: db_name.clone(),
        admin_url: admin_url.to_string(),
    };
    guard.insert(
        key,
        TemplateDb {
            db_name: db_name.clone(),
            admin_url: admin_url.to_string(),
        },
    );
    Ok(entry)
}

/// 4-byte hex of SHA-256(canonical path or "none").
fn template_key(migrations_dir: Option<&Path>) -> anyhow::Result<String> {
    let canon = match migrations_dir {
        Some(p) => p
            .canonicalize()
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from("none")),
        None => String::from("none"),
    };
    let mut h = Sha256::new();
    h.update(canon.as_bytes());
    let digest = h.finalize();
    Ok(hex::encode(&digest[..4]))
}

/// Replace the database-name segment of a `postgres://` URL.
///
/// Handles both `postgres://host/db` and `postgres://host:port/db` forms.
/// Returns the original string unchanged when no `/db` suffix is found.
fn with_db(admin_url: &str, db_name: &str) -> String {
    // Find the last `/` after the authority section.
    // A proper postgres URL has the form `postgres://auth/dbname[?opts]`.
    if let Some(after_proto) = admin_url
        .strip_prefix("postgres://")
        .or_else(|| admin_url.strip_prefix("postgresql://"))
    {
        // The authority ends at the first `/` after the scheme.
        if let Some(slash_pos) = after_proto.find('/') {
            let prefix_len = admin_url.len() - after_proto.len() + slash_pos; // position of that slash
            let after_slash = &admin_url[prefix_len + 1..];
            // Strip any existing query string from the old db name.
            let qmark = after_slash.find('?').unwrap_or(after_slash.len());
            let query_part = &after_slash[qmark..]; // "" or "?..."
            return format!("{}/{db_name}{query_part}", &admin_url[..prefix_len]);
        }
    }
    // Fallback: append if we couldn't locate the path.
    format!("{admin_url}/{db_name}")
}

async fn create_template(
    admin_url: &str,
    db_name: &str,
    migrations_dir: Option<&Path>,
) -> anyhow::Result<()> {
    // ------------------------------------------------------------------
    // 1. Drop any stale copy and create a fresh empty database.
    // ------------------------------------------------------------------
    {
        let cfg = Config::parse(admin_url)?;
        let mut admin = Connection::connect(cfg).await?;
        admin
            .execute("SET client_min_messages = ERROR", &[])
            .await?;
        admin
            .execute(&format!("DROP DATABASE IF EXISTS {db_name}"), &[])
            .await?;
        admin
            .execute(&format!("CREATE DATABASE {db_name}"), &[])
            .await?;
    }

    // ------------------------------------------------------------------
    // 2. Run migrations into the new database (if a directory was given).
    // ------------------------------------------------------------------
    if let Some(dir) = migrations_dir {
        let tmpl_url = with_db(admin_url, db_name);
        let tmpl_cfg = Config::parse(&tmpl_url)?;
        let pool = Pool::new(tmpl_cfg, PoolConfig::new().max_connections(2));
        let migrator = sntl_migrate::Migrator::from_dir(dir)?;
        migrator.run(&pool).await?;
    }

    // ------------------------------------------------------------------
    // 3. Mark the database as a PostgreSQL template so cloning is instant.
    // ------------------------------------------------------------------
    {
        let cfg = Config::parse(admin_url)?;
        let mut admin = Connection::connect(cfg).await?;
        admin
            .execute(
                &format!(
                    "UPDATE pg_database SET datistemplate = true \
                     WHERE datname = '{db_name}'"
                ),
                &[],
            )
            .await?;
    }

    Ok(())
}

/// Clone `template` into a fresh database named `_sntl_{suffix}`.
///
/// The clone is near-instant because PostgreSQL copies the template at the
/// storage layer. The `suffix` must already be sanitised by the caller
/// (see `run::run`).
pub async fn clone_into(template: &TemplateDb, suffix: &str) -> anyhow::Result<String> {
    let new_db = format!("_sntl_{suffix}");
    let cfg = Config::parse(&template.admin_url)?;
    let mut admin = Connection::connect(cfg).await?;
    admin
        .execute("SET client_min_messages = ERROR", &[])
        .await?;
    // DDL cannot be parameterised; new_db and template.db_name are safe
    // (hex digest / `_sntl_`-prefixed sanitised suffix).
    admin
        .execute(
            &format!("CREATE DATABASE {new_db} TEMPLATE {}", template.db_name),
            &[],
        )
        .await?;
    Ok(new_db)
}

/// Drop a per-test database. Best-effort: logs nothing, never panics.
pub async fn drop_db(admin_url: &str, db_name: &str) {
    let cfg = match Config::parse(admin_url) {
        Ok(c) => c,
        Err(_) => return,
    };
    if let Ok(mut admin) = Connection::connect(cfg).await {
        let _ = admin.execute("SET client_min_messages = ERROR", &[]).await;
        let _ = admin
            .execute(&format!("DROP DATABASE IF EXISTS {db_name}"), &[])
            .await;
    }
}
