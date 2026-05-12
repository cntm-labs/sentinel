# `sntl-migrate` v0.3 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the `sntl-migrate` library + CLI subcommands implementing forward-only migrations with the unique-to-Sentinel cache-vs-DB diff scaffolder. Closes the largest user-facing parity gap with sqlx/SeaORM/Diesel.

**Architecture:** Library crate `sntl-migrate` (replaces the existing name-reservation stub) owns the runtime logic. `sntl-macros` adds the `migrate!()` compile-time bundling macro. `sntl-cli` adds `migrate add/run/info/diff/verify` subcommands as a thin wrapper. PostgreSQL advisory lock (i64 keyed) serialises concurrent applies. Each migration runs in its own transaction unless the file is named `up.notx.sql`.

**Tech Stack:** Rust 1.85 / edition 2024, syn 2, quote 1, proc-macro-error2, sha2 + hex (checksum), walkdir, thiserror, sentinel-driver 2.0 (`PgAdvisoryLock`, `Connection`, `Pool`), sntl-schema (for `Schema` struct + `pull_schema` introspect), trybuild.

---

## Reference material

- Design: `docs/plans/2026-05-09-sntl-migrate-design.md` — read §3 (CLI), §4 (diff algorithm), and §5 (error UX) before starting.
- Template style: `docs/plans/2026-04-20-sntl-query-macro-impl.md` + `docs/plans/2026-04-29-cluster-a-array-tuple-impl.md` set the per-task TDD format used here.

### Open items resolved before writing this plan

1. `sentinel-driver` exposes `PgAdvisoryLock::new(i64)` + `acquire(&mut Connection) -> PgAdvisoryLockGuard`. **`PgAdvisoryLockGuard` is NOT auto-released on drop** — must call `guard.release(conn).await`. Plan uses an explicit `try/release` flow.
2. `sntl-schema::Schema/Table/Column` have `primary_key`, `unique`, `default`, `foreign_keys` fields. `pull_schema` populates pk + unique but leaves `foreign_keys: vec![]`. **FK diff is out of v0.3 scope** — defer with the composite/enum work in roadmap sub-project #4.
3. Lock ID constant: `0x736e_746c_6d67_7274_i64` = ASCII "sntlmgrt".

---

## File structure (master map)

### New library crate: `sntl-migrate/`

```
sntl-migrate/
├── Cargo.toml                                # MODIFY: replace stub
├── src/
│   ├── lib.rs                                # public surface
│   ├── error.rs                              # typed errors
│   ├── migration.rs                          # Migration + Version + TxMode
│   ├── discover.rs                           # folder walk → Vec<Migration>
│   ├── checksum.rs                           # sha256 hash helper
│   ├── tracking.rs                           # _sntl_migrations table I/O
│   ├── runner.rs                             # apply loop + advisory lock
│   ├── refresh.rs                            # post-apply schema.toml refresh
│   ├── macro_support.rs                      # Migrator::from_static helpers
│   └── diff/
│       ├── mod.rs
│       ├── compare.rs                        # Schema vs Schema → Vec<Change>
│       └── emit.rs                           # Vec<Change> → SQL skeleton
└── tests/
    ├── version_test.rs                       # parse / order
    ├── discover_test.rs                      # folder walk
    ├── checksum_test.rs
    ├── tracking_test.rs                      # live-PG
    ├── runner_test.rs                        # live-PG: apply / lock / rerun
    ├── diff_test.rs                          # compare + emit
    └── embedded_test.rs                      # macro bundling
```

### Extended crate: `sntl-macros/`

```
sntl-macros/src/
├── lib.rs                                    # MODIFY: register migrate!
└── migrate/
    ├── mod.rs                                # NEW
    └── codegen.rs                            # NEW: emit Migrator::from_static
```

### Extended crate: `sntl-cli/`

```
sntl-cli/src/commands/
├── mod.rs                                    # MODIFY: pub mod migrate
└── migrate.rs                                # NEW: add/run/info/diff/verify
```

### Workspace root

```
Cargo.toml                                    # MODIFY: keep sntl-migrate path dep
docs/migration-guide.md                       # NEW: user guide
```

---

## Phase 0 — Scaffolding

### Task 1: Replace `sntl-migrate` stub Cargo.toml + lib.rs

**Files:**
- Modify: `sntl-migrate/Cargo.toml`
- Create: `sntl-migrate/src/lib.rs` (replacing existing stub)

- [ ] **Step 1: Replace `sntl-migrate/Cargo.toml`**

```toml
[package]
name = "sntl-migrate"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Migrations library + CLI helpers for Sentinel ORM"
readme = "../README.md"
keywords = ["orm", "postgresql", "migrations"]
categories = ["database"]

[dependencies]
sentinel-driver.workspace = true
sntl-schema = { workspace = true, features = ["online"] }
thiserror.workspace = true
serde.workspace = true
tokio.workspace = true
sha2.workspace = true
hex.workspace = true
walkdir.workspace = true

[dev-dependencies]
tempfile.workspace = true
chrono.workspace = true
```

- [ ] **Step 2: Replace `sntl-migrate/src/lib.rs`**

```rust
//! Forward-only SQL migrations for Sentinel ORM.
//!
//! Two entry points:
//! - `Migrator::from_dir("migrations/")` for the CLI and dev workflows.
//! - `sntl_migrate::migrate!("./migrations")` (re-exported from `sntl-macros`)
//!   for compile-time embedding into a production binary.
//!
//! See `docs/migration-guide.md` for the full user guide.

pub mod checksum;
pub mod diff;
pub mod discover;
pub mod error;
pub mod macro_support;
pub mod migration;
pub mod refresh;
pub mod runner;
pub mod tracking;

pub use error::{Error, Result};
pub use migration::{Migration, TxMode, Version};
pub use runner::{MigrationReport, MigrationStatus, Migrator};

/// The PostgreSQL advisory-lock ID used to serialise concurrent migrators.
/// ASCII bytes "sntlmgrt" — chosen to be unlikely to collide with other tools.
pub const SNTL_MIGRATE_LOCK_ID: i64 = 0x736e_746c_6d67_7274_i64;
```

- [ ] **Step 3: Create empty module stubs**

```bash
for f in checksum diff/mod discover error macro_support migration refresh runner tracking; do
    mkdir -p "sntl-migrate/src/$(dirname "$f")"
    printf "//! %s — populated in a later task.\n" "$f" > "sntl-migrate/src/$f.rs"
done
```

- [ ] **Step 4: Verify workspace compiles**

Run: `cargo check -p sntl-migrate 2>&1 | tail -5`
Expected: errors about unresolved imports from the `pub use` block.

- [ ] **Step 5: Replace stub re-exports with placeholders**

In `sntl-migrate/src/lib.rs`, comment out the `pub use` lines temporarily — they will be uncommented once Tasks 2-3 land the types.

- [ ] **Step 6: Commit**

```bash
git add sntl-migrate/
git commit -m "chore(sntl-migrate): scaffold crate (modules + Cargo.toml + lock ID const)"
```

---

## Phase 1 — Library core (PR-1)

### Task 2: `error.rs` — typed Error enum

**Files:**
- Modify: `sntl-migrate/src/error.rs`

- [ ] **Step 1: Write enum**

```rust
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
```

- [ ] **Step 2: Verify compile**

Run: `cargo check -p sntl-migrate 2>&1 | tail -3`
Expected: errors only from the still-empty modules referenced in lib.rs.

- [ ] **Step 3: Commit**

```bash
git add sntl-migrate/src/error.rs
git commit -m "feat(sntl-migrate): typed Error enum for migration failures"
```

---

### Task 3: `migration.rs` — Migration + Version + TxMode

**Files:**
- Modify: `sntl-migrate/src/migration.rs`
- Create: `sntl-migrate/tests/version_test.rs`

- [ ] **Step 1: Write failing test**

Create `sntl-migrate/tests/version_test.rs`:

```rust
use sntl_migrate::migration::Version;

#[test]
fn parses_valid_folder_name() {
    let v: Version = "20260509_140000_add_users".parse().unwrap();
    assert_eq!(v.timestamp(), "20260509_140000");
    assert_eq!(v.name(), "add_users");
    assert_eq!(v.as_str(), "20260509_140000_add_users");
}

#[test]
fn rejects_short_timestamp() {
    assert!("2026_add_users".parse::<Version>().is_err());
}

#[test]
fn rejects_missing_name() {
    assert!("20260509_140000".parse::<Version>().is_err());
}

#[test]
fn ordering_by_timestamp() {
    let a: Version = "20260509_140000_a".parse().unwrap();
    let b: Version = "20260510_080000_b".parse().unwrap();
    assert!(a < b);
}
```

- [ ] **Step 2: Run, confirm failure**

Run: `cargo test -p sntl-migrate --test version_test`
Expected: FAIL (Version undefined).

- [ ] **Step 3: Implement `migration.rs`**

```rust
use std::fmt;
use std::str::FromStr;

use crate::error::Error;

/// Transaction mode for a migration file.
///
/// Default `PerMigration` wraps each migration in `BEGIN/COMMIT`. Migrations
/// with non-transactional DDL (`CREATE INDEX CONCURRENTLY`, `VACUUM`, etc.)
/// can declare `up.notx.sql` instead of `up.sql`, which maps to `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxMode {
    PerMigration,
    None,
}

/// A migration's identifier — `YYYYMMDD_HHMMSS_<snake_case_name>`.
///
/// Lexicographic ordering matches chronological ordering since the timestamp
/// is fixed-width.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(String);

impl Version {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn timestamp(&self) -> &str {
        // safe: we validated YYYYMMDD_HHMMSS = 15 chars
        &self.0[..15]
    }

    pub fn name(&self) -> &str {
        &self.0[16..]
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 17 {
            return Err(Error::InvalidName { name: s.to_string() });
        }
        // First 8 = date digits, then '_', then 6 hour-digits, then '_', then name.
        let date = &s[0..8];
        let sep1 = &s[8..9];
        let time = &s[9..15];
        let sep2 = &s[15..16];
        if !date.chars().all(|c| c.is_ascii_digit())
            || sep1 != "_"
            || !time.chars().all(|c| c.is_ascii_digit())
            || sep2 != "_"
        {
            return Err(Error::InvalidName { name: s.to_string() });
        }
        let name = &s[16..];
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(Error::InvalidName { name: s.to_string() });
        }
        Ok(Self(s.to_string()))
    }
}

/// A single discovered migration: identifier, SQL text, and tx mode.
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: Version,
    pub sql: String,
    pub tx_mode: TxMode,
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cargo test -p sntl-migrate --test version_test`
Expected: 4 passed.

- [ ] **Step 5: Uncomment re-exports in `lib.rs`**

In `sntl-migrate/src/lib.rs`, restore the originally-planned `pub use migration::{Migration, TxMode, Version};` line. Leave `runner::*` commented for now.

- [ ] **Step 6: Commit**

```bash
git add sntl-migrate/src/{lib,migration}.rs sntl-migrate/tests/version_test.rs
git commit -m "feat(sntl-migrate): Migration + Version + TxMode with FromStr parser"
```

---

### Task 4: `checksum.rs` — sha256 of SQL content

**Files:**
- Modify: `sntl-migrate/src/checksum.rs`
- Create: `sntl-migrate/tests/checksum_test.rs`

- [ ] **Step 1: Failing test**

```rust
use sntl_migrate::checksum::sha256_of_sql;

#[test]
fn deterministic() {
    let a = sha256_of_sql("CREATE TABLE foo (id int);");
    let b = sha256_of_sql("CREATE TABLE foo (id int);");
    assert_eq!(a, b);
}

#[test]
fn sensitive_to_whitespace() {
    let a = sha256_of_sql("CREATE TABLE foo (id int);");
    let b = sha256_of_sql("CREATE  TABLE foo (id int);");
    assert_ne!(a, b, "whitespace difference must change the hash");
}

#[test]
fn truncated_length_is_13() {
    let h = sha256_of_sql("anything");
    assert_eq!(h.len(), 13);
}
```

- [ ] **Step 2: Run, expect FAIL**

Run: `cargo test -p sntl-migrate --test checksum_test`

- [ ] **Step 3: Implement `checksum.rs`**

```rust
use sha2::{Digest, Sha256};

/// Compute a stable short hash of the migration SQL text.
///
/// Used to detect "applied migration file was modified after apply".
/// 13-char prefix matches the `.sentinel/queries/<hash>.json` format
/// chosen by `sntl-schema::normalize`.
pub fn sha256_of_sql(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    hex::encode(&digest[..7])[..13].to_string()
}
```

- [ ] **Step 4: Run, expect 3 passed**
- [ ] **Step 5: Commit**

```bash
git add sntl-migrate/src/checksum.rs sntl-migrate/tests/checksum_test.rs
git commit -m "feat(sntl-migrate): sha256 checksum helper for migration content"
```

---

### Task 5: `discover.rs` — walk `migrations/` directory

**Files:**
- Modify: `sntl-migrate/src/discover.rs`
- Create: `sntl-migrate/tests/discover_test.rs`

- [ ] **Step 1: Failing test**

```rust
use sntl_migrate::discover::discover;
use std::fs;
use tempfile::tempdir;

fn touch(dir: &std::path::Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[test]
fn empty_dir_returns_empty_vec() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("migrations")).unwrap();
    let m = discover(&dir.path().join("migrations")).unwrap();
    assert!(m.is_empty());
}

#[test]
fn finds_and_sorts_two_migrations() {
    let dir = tempdir().unwrap();
    touch(dir.path(), "migrations/20260510_080000_b/up.sql", "SELECT 2;");
    touch(dir.path(), "migrations/20260509_140000_a/up.sql", "SELECT 1;");
    let m = discover(&dir.path().join("migrations")).unwrap();
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].version.name(), "a");
    assert_eq!(m[1].version.name(), "b");
}

#[test]
fn detects_up_notx_variant() {
    let dir = tempdir().unwrap();
    touch(dir.path(), "migrations/20260509_140000_idx/up.notx.sql", "CREATE INDEX CONCURRENTLY ...");
    let m = discover(&dir.path().join("migrations")).unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].tx_mode, sntl_migrate::TxMode::None);
}

#[test]
fn rejects_malformed_folder() {
    let dir = tempdir().unwrap();
    touch(dir.path(), "migrations/not_a_version/up.sql", "");
    let err = discover(&dir.path().join("migrations")).unwrap_err();
    assert!(matches!(err, sntl_migrate::Error::InvalidName { .. }));
}

#[test]
fn missing_dir_returns_missing_error() {
    let dir = tempdir().unwrap();
    let err = discover(&dir.path().join("nope")).unwrap_err();
    assert!(matches!(err, sntl_migrate::Error::MissingDir { .. }));
}
```

- [ ] **Step 2: Run, expect FAIL**

- [ ] **Step 3: Implement `discover.rs`**

```rust
use std::path::Path;

use crate::error::{Error, Result};
use crate::migration::{Migration, TxMode, Version};

/// Walk the `migrations/` directory, parse each folder as a `Version`, and
/// return migrations in ascending version order.
pub fn discover(migrations_dir: &Path) -> Result<Vec<Migration>> {
    if !migrations_dir.exists() {
        return Err(Error::MissingDir { path: migrations_dir.to_path_buf() });
    }
    if !migrations_dir.is_dir() {
        return Err(Error::MissingDir { path: migrations_dir.to_path_buf() });
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
        out.push(Migration { version, sql, tx_mode });
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
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "neither up.sql nor up.notx.sql found"),
    })
}
```

- [ ] **Step 4: Run, expect 5 passed**

- [ ] **Step 5: Commit**

```bash
git add sntl-migrate/src/discover.rs sntl-migrate/tests/discover_test.rs
git commit -m "feat(sntl-migrate): discover walks migrations/ + detects up.notx.sql"
```

---

### Task 6: `tracking.rs` — `_sntl_migrations` table I/O (live PG)

**Files:**
- Modify: `sntl-migrate/src/tracking.rs`
- Create: `sntl-migrate/tests/tracking_test.rs`

- [ ] **Step 1: Failing test**

```rust
//! Live-PG tests for the tracking table. Skips when DATABASE_URL is unset.

use sntl_migrate::migration::Version;
use sntl_migrate::tracking;

async fn connect() -> Option<sentinel_driver::Connection> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = sentinel_driver::Config::parse(&url).ok()?;
    sentinel_driver::Connection::connect(cfg).await.ok()
}

#[tokio::test]
async fn ensure_is_idempotent() {
    let Some(mut conn) = connect().await else { return };
    tracking::drop_table(&mut conn).await.ok();
    tracking::ensure(&mut conn).await.unwrap();
    tracking::ensure(&mut conn).await.unwrap();
}

#[tokio::test]
async fn record_and_load_round_trip() {
    let Some(mut conn) = connect().await else { return };
    tracking::drop_table(&mut conn).await.ok();
    tracking::ensure(&mut conn).await.unwrap();
    let v: Version = "20260509_140000_a".parse().unwrap();
    tracking::record(&mut conn, &v, "abc123").await.unwrap();
    let applied = tracking::applied(&mut conn).await.unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].0, v);
    assert_eq!(applied[0].1, "abc123");
}
```

- [ ] **Step 2: Run (skips without DB; pass when DB set)**

- [ ] **Step 3: Implement `tracking.rs`**

```rust
use sentinel_driver::Connection;

use crate::error::Result;
use crate::migration::Version;

const TABLE_NAME: &str = "_sntl_migrations";

/// Create the tracking table if it does not exist. Idempotent.
pub async fn ensure(conn: &mut Connection) -> Result<()> {
    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {TABLE_NAME} (\
                version    text PRIMARY KEY,\
                applied_at timestamptz NOT NULL DEFAULT now(),\
                checksum   text NOT NULL\
            )"
        ),
        &[],
    )
    .await?;
    Ok(())
}

/// Drop the tracking table. Test helper only.
#[doc(hidden)]
pub async fn drop_table(conn: &mut Connection) -> Result<()> {
    conn.execute(&format!("DROP TABLE IF EXISTS {TABLE_NAME}"), &[]).await?;
    Ok(())
}

/// Return all applied migrations as `(version, checksum)` ordered by version.
pub async fn applied(conn: &mut Connection) -> Result<Vec<(Version, String)>> {
    let rows = conn
        .query(
            &format!("SELECT version, checksum FROM {TABLE_NAME} ORDER BY version ASC"),
            &[],
        )
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let v: String = row.try_get(0)?;
        let cs: String = row.try_get(1)?;
        let version: Version = v.parse()?;
        out.push((version, cs));
    }
    Ok(out)
}

/// Insert a successfully-applied migration record.
pub async fn record(conn: &mut Connection, version: &Version, checksum: &str) -> Result<()> {
    conn.execute(
        &format!("INSERT INTO {TABLE_NAME} (version, checksum) VALUES ($1, $2)"),
        &[&version.as_str(), &checksum],
    )
    .await?;
    Ok(())
}
```

- [ ] **Step 4: Run with live PG**

Run: `DATABASE_URL=postgres://… cargo test -p sntl-migrate --test tracking_test`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add sntl-migrate/src/tracking.rs sntl-migrate/tests/tracking_test.rs
git commit -m "feat(sntl-migrate): _sntl_migrations table ensure/applied/record"
```

---

### Task 7: `runner.rs` — advisory-lock + apply loop

**Files:**
- Modify: `sntl-migrate/src/runner.rs`
- Modify: `sntl-migrate/src/lib.rs` (uncomment `runner` re-exports)
- Create: `sntl-migrate/tests/runner_test.rs`

- [ ] **Step 1: Write tests (live PG)**

```rust
use sntl_migrate::{Migrator, TxMode, Version};
use tempfile::tempdir;
use std::fs;

async fn pool() -> Option<sentinel_driver::Pool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = sentinel_driver::Config::parse(&url).ok()?;
    Some(sentinel_driver::Pool::new(cfg, sentinel_driver::pool::config::PoolConfig::new().max_connections(4)))
}

fn write_mig(root: &std::path::Path, version: &str, sql: &str) {
    let dir = root.join("migrations").join(version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("up.sql"), sql).unwrap();
}

#[tokio::test]
async fn applies_pending_in_order_then_noop() {
    let Some(pool) = pool().await else { return };
    let mut admin = pool.acquire().await.unwrap();
    admin.execute("DROP TABLE IF EXISTS _sntl_migrations", &[]).await.unwrap();
    admin.execute("DROP TABLE IF EXISTS runner_test", &[]).await.unwrap();
    drop(admin);

    let dir = tempdir().unwrap();
    write_mig(dir.path(), "20260509_140000_create", "CREATE TABLE runner_test (id int);");
    write_mig(dir.path(), "20260509_150000_insert", "INSERT INTO runner_test (id) VALUES (1);");
    let migrator = Migrator::from_dir(dir.path().join("migrations")).unwrap();
    let first = migrator.run(&pool).await.unwrap();
    assert_eq!(first.applied.len(), 2);
    let second = migrator.run(&pool).await.unwrap();
    assert!(second.applied.is_empty(), "second run must be no-op");
}

#[tokio::test]
async fn out_of_order_errors() {
    let Some(pool) = pool().await else { return };
    let mut admin = pool.acquire().await.unwrap();
    admin.execute("DROP TABLE IF EXISTS _sntl_migrations", &[]).await.unwrap();
    drop(admin);

    let dir = tempdir().unwrap();
    write_mig(dir.path(), "20260510_080000_b", "SELECT 1;");
    let m1 = Migrator::from_dir(dir.path().join("migrations")).unwrap();
    m1.run(&pool).await.unwrap();
    // Now add an earlier-timestamp migration
    write_mig(dir.path(), "20260509_080000_a", "SELECT 1;");
    let m2 = Migrator::from_dir(dir.path().join("migrations")).unwrap();
    let err = m2.run(&pool).await.unwrap_err();
    assert!(matches!(err, sntl_migrate::Error::OutOfOrder { .. }));
}

#[tokio::test]
async fn notx_runs_outside_transaction() {
    let Some(pool) = pool().await else { return };
    let mut admin = pool.acquire().await.unwrap();
    admin.execute("DROP TABLE IF EXISTS _sntl_migrations", &[]).await.unwrap();
    admin.execute("DROP TABLE IF EXISTS notx_test", &[]).await.unwrap();
    admin.execute("CREATE TABLE notx_test (id int)", &[]).await.unwrap();
    drop(admin);

    let dir = tempdir().unwrap();
    let mig = dir.path().join("migrations/20260509_140000_idx");
    fs::create_dir_all(&mig).unwrap();
    // CREATE INDEX CONCURRENTLY cannot run inside transaction
    fs::write(mig.join("up.notx.sql"), "CREATE INDEX CONCURRENTLY notx_idx ON notx_test (id);").unwrap();
    Migrator::from_dir(dir.path().join("migrations")).unwrap().run(&pool).await.unwrap();
}
```

- [ ] **Step 2: Implement `runner.rs`**

```rust
use std::path::{Path, PathBuf};

use sentinel_driver::advisory_lock::PgAdvisoryLock;
use sentinel_driver::{Connection, Pool};

use crate::checksum::sha256_of_sql;
use crate::discover::discover;
use crate::error::{Error, Result};
use crate::migration::{Migration, TxMode, Version};
use crate::tracking;
use crate::SNTL_MIGRATE_LOCK_ID;

/// Result of a single `Migrator::run` invocation.
#[derive(Debug, Default)]
pub struct MigrationReport {
    pub applied: Vec<Version>,
}

/// One row in `sntl migrate info`.
#[derive(Debug)]
pub struct MigrationStatus {
    pub version: Version,
    pub state: State,
    pub checksum: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Applied,
    Pending,
    ChecksumDrift,
}

/// Top-level migration runner.
pub struct Migrator {
    migrations: Vec<Migration>,
    source: MigrationSource,
}

#[derive(Debug)]
enum MigrationSource {
    Dir(PathBuf),
    Static,
}

impl Migrator {
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let migrations = discover(&path)?;
        Ok(Self { migrations, source: MigrationSource::Dir(path) })
    }

    pub fn from_static(entries: &'static [(&'static str, &'static str, TxMode)]) -> Self {
        let migrations = entries
            .iter()
            .map(|(v, sql, mode)| Migration {
                version: v.parse().expect("compile-time migration version must be valid"),
                sql: (*sql).to_string(),
                tx_mode: *mode,
            })
            .collect();
        Self { migrations, source: MigrationSource::Static }
    }

    pub async fn run(&self, pool: &Pool) -> Result<MigrationReport> {
        let mut conn = pool.acquire().await?;
        let lock = PgAdvisoryLock::new(SNTL_MIGRATE_LOCK_ID);
        let guard = lock.acquire(&mut conn).await?;

        let result = self.run_locked(&mut conn).await;

        // Release lock before returning. If release fails, surface that.
        guard.release(&mut conn).await?;
        result
    }

    async fn run_locked(&self, conn: &mut Connection) -> Result<MigrationReport> {
        tracking::ensure(conn).await?;
        let applied = tracking::applied(conn).await?;
        let applied_set: std::collections::BTreeSet<Version> =
            applied.iter().map(|(v, _)| v.clone()).collect();
        let highest_applied = applied_set.iter().max().cloned();

        let mut report = MigrationReport::default();
        for m in &self.migrations {
            if applied_set.contains(&m.version) {
                continue;
            }
            // Strict ordering: pending migration cannot precede any applied.
            if let Some(highest) = &highest_applied {
                if m.version < *highest {
                    return Err(Error::OutOfOrder {
                        pending: m.version.clone(),
                        highest_applied: highest.clone(),
                    });
                }
            }

            apply_one(conn, m).await?;
            tracking::record(conn, &m.version, &sha256_of_sql(&m.sql)).await?;
            report.applied.push(m.version.clone());
        }
        Ok(report)
    }

    pub async fn info(&self, pool: &Pool) -> Result<Vec<MigrationStatus>> {
        let mut conn = pool.acquire().await?;
        tracking::ensure(&mut conn).await?;
        let applied = tracking::applied(&mut conn).await?;
        let applied_map: std::collections::BTreeMap<Version, String> =
            applied.into_iter().collect();

        let mut out = Vec::with_capacity(self.migrations.len() + applied_map.len());
        for m in &self.migrations {
            if let Some(recorded) = applied_map.get(&m.version) {
                let current = sha256_of_sql(&m.sql);
                let state = if &current == recorded {
                    State::Applied
                } else {
                    State::ChecksumDrift
                };
                out.push(MigrationStatus {
                    version: m.version.clone(),
                    state,
                    checksum: Some(recorded.clone()),
                });
            } else {
                out.push(MigrationStatus {
                    version: m.version.clone(),
                    state: State::Pending,
                    checksum: None,
                });
            }
        }
        Ok(out)
    }

    pub fn migrations(&self) -> &[Migration] {
        &self.migrations
    }

    pub fn source_path(&self) -> Option<&Path> {
        match &self.source {
            MigrationSource::Dir(p) => Some(p.as_path()),
            MigrationSource::Static => None,
        }
    }
}

async fn apply_one(conn: &mut Connection, m: &Migration) -> Result<()> {
    match m.tx_mode {
        TxMode::PerMigration => {
            conn.execute("BEGIN", &[]).await?;
            if let Err(e) = conn.execute(&m.sql, &[]).await {
                conn.execute("ROLLBACK", &[]).await.ok();
                return Err(Error::ApplyFailed { version: m.version.clone(), source: e });
            }
            conn.execute("COMMIT", &[]).await
                .map_err(|source| Error::ApplyFailed { version: m.version.clone(), source })?;
        }
        TxMode::None => {
            conn.execute(&m.sql, &[]).await
                .map_err(|source| Error::ApplyFailed { version: m.version.clone(), source })?;
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Uncomment re-exports**

In `sntl-migrate/src/lib.rs` restore `pub use runner::{MigrationReport, MigrationStatus, Migrator};` and also add `pub use runner::State;`.

- [ ] **Step 4: Run tests**

Run: `DATABASE_URL=… cargo test -p sntl-migrate --test runner_test -- --test-threads=1`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add sntl-migrate/src/{runner,lib}.rs sntl-migrate/tests/runner_test.rs
git commit -m "feat(sntl-migrate): Migrator with advisory lock + per-migration tx + strict ordering"
```

---

### Task 8: `refresh.rs` — post-apply schema.toml refresh

**Files:**
- Modify: `sntl-migrate/src/refresh.rs`
- Modify: `sntl-migrate/src/runner.rs` (call refresh after apply)

- [ ] **Step 1: Implement `refresh.rs`**

```rust
use std::path::Path;

use crate::error::Result;

/// Pull the live schema and write it to `.sentinel/schema.toml`.
///
/// Called by `Migrator::run` after migrations apply so the compile-time
/// `query!()` cache always reflects the new schema. The caller controls
/// the cache directory.
pub async fn refresh_schema(conn_str: &str, cache_dir: &Path) -> Result<()> {
    let schema = sntl_schema::introspect::pull_schema(conn_str).await?;
    let cache = sntl_schema::cache::Cache::new(cache_dir);
    cache.init()?;
    cache.write_schema(&schema)?;
    Ok(())
}
```

- [ ] **Step 2: Extend `Migrator::run` to call refresh**

In `runner.rs`, add an `Option<RefreshConfig>` field to `Migrator` and a builder method `with_refresh(conn_str: String, cache_dir: PathBuf)`. After `run_locked` returns successfully, call `refresh::refresh_schema` if configured. (The CLI sets this; library callers opt in.)

Add at bottom of `runner.rs`:

```rust
#[derive(Debug, Clone)]
pub struct RefreshConfig {
    pub conn_str: String,
    pub cache_dir: std::path::PathBuf,
}

impl Migrator {
    pub fn with_refresh(mut self, conn_str: impl Into<String>, cache_dir: impl Into<std::path::PathBuf>) -> Self {
        self.refresh = Some(RefreshConfig { conn_str: conn_str.into(), cache_dir: cache_dir.into() });
        self
    }
}
```

Add `refresh: Option<RefreshConfig>` to the `Migrator` struct (initialise to `None` in both constructors).

In `run`, after `guard.release(&mut conn).await?;` and the `result` is `Ok`, do:

```rust
if let Some(cfg) = &self.refresh {
    crate::refresh::refresh_schema(&cfg.conn_str, &cfg.cache_dir).await?;
}
```

- [ ] **Step 3: Build**

Run: `cargo check -p sntl-migrate`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add sntl-migrate/src/{refresh,runner}.rs
git commit -m "feat(sntl-migrate): auto-refresh .sentinel/schema.toml after apply"
```

---

### Task 9: PR-1 wrap-up — workspace cargo deny + clippy + open PR

- [ ] **Step 1: Register `sntl-migrate` properly in workspace**

In root `Cargo.toml` `[workspace.dependencies]` block, confirm `sntl-migrate = { path = "sntl-migrate", version = "0.1.0" }` is present and uncommented (it was added with the original name reservation; leave version at 0.1.0).

- [ ] **Step 2: Run full workspace verification**

```bash
cargo check --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo fmt --all -- --check
DATABASE_URL=… cargo test --workspace --all-features -- --test-threads=1
cargo deny check
```

All must pass.

- [ ] **Step 3: Push + open PR-1**

```bash
git push -u origin feat/sntl-migrate
gh pr create --title "feat(sntl-migrate): library core — tracking + runner + lock + refresh" --body "..."
```

PR body summarises Tasks 1–8 and links the design doc.

---

## Phase 2 — Diff + macro + CLI (PR-2)

### Task 10: `diff::compare` — Schema vs Schema → Vec<Change>

**Files:**
- Modify: `sntl-migrate/src/diff/mod.rs`
- Create: `sntl-migrate/src/diff/compare.rs`
- Create: `sntl-migrate/tests/diff_test.rs` (initial — extended in Task 11)

- [ ] **Step 1: Define `Change` enum in `diff/mod.rs`**

```rust
pub mod compare;
pub mod emit;

pub use compare::{compare, Change};
pub use emit::emit;
```

- [ ] **Step 2: Implement `diff/compare.rs`**

```rust
use sntl_schema::schema::{Column, Schema, Table};

/// All structural diffs between two schemas. Ordering is meaningful for
/// emit: dependencies first (CREATE TABLE before its columns get touched).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    AddTable(Table),
    DropTable { name: String },
    AddColumn  { table: String, column: Column },
    DropColumn { table: String, column: String },
    AlterColumnType { table: String, column: String, from: String, to: String },
    AlterColumnNullable { table: String, column: String, to: bool },
    AlterColumnDefault  { table: String, column: String, to: Option<String> },
    AddPrimaryKey  { table: String, columns: Vec<String> },
    DropPrimaryKey { table: String },
    AddUnique      { table: String, columns: Vec<String> },
    DropUnique     { table: String, columns: Vec<String> },
}

/// Compute `target_state` - `current_state` in terms of executable Changes.
///
/// `cache` = the desired state (committed `.sentinel/schema.toml`).
/// `live`  = what the DB currently shows.
///
/// FK changes are **out of v0.3 scope** — pull_schema doesn't populate them.
pub fn compare(cache: &Schema, live: &Schema) -> Vec<Change> {
    let mut out = Vec::new();

    // Tables in cache but not in live → AddTable
    for t in &cache.tables {
        if live.find_table(&t.name).is_none() {
            out.push(Change::AddTable(t.clone()));
        }
    }
    // Tables in live but not in cache → DropTable
    for t in &live.tables {
        if cache.find_table(&t.name).is_none() {
            out.push(Change::DropTable { name: t.name.clone() });
        }
    }
    // Tables present in both → column diff
    for cache_t in &cache.tables {
        let Some(live_t) = live.find_table(&cache_t.name) else { continue };
        diff_columns(cache_t, live_t, &mut out);
        diff_pk(cache_t, live_t, &mut out);
        diff_unique(cache_t, live_t, &mut out);
    }

    out
}

fn diff_columns(cache_t: &Table, live_t: &Table, out: &mut Vec<Change>) {
    // Added columns
    for c in &cache_t.columns {
        if live_t.columns.iter().any(|lc| lc.name == c.name) { continue; }
        out.push(Change::AddColumn { table: cache_t.name.clone(), column: c.clone() });
    }
    // Removed columns
    for c in &live_t.columns {
        if cache_t.columns.iter().any(|cc| cc.name == c.name) { continue; }
        out.push(Change::DropColumn { table: cache_t.name.clone(), column: c.name.clone() });
    }
    // Type / nullable / default diffs on shared columns
    for cc in &cache_t.columns {
        let Some(lc) = live_t.columns.iter().find(|lc| lc.name == cc.name) else { continue };
        if cc.pg_type.pg_type != lc.pg_type.pg_type {
            out.push(Change::AlterColumnType {
                table: cache_t.name.clone(),
                column: cc.name.clone(),
                from: lc.pg_type.pg_type.clone(),
                to: cc.pg_type.pg_type.clone(),
            });
        }
        if cc.nullable != lc.nullable {
            out.push(Change::AlterColumnNullable {
                table: cache_t.name.clone(),
                column: cc.name.clone(),
                to: cc.nullable,
            });
        }
        if cc.default != lc.default {
            out.push(Change::AlterColumnDefault {
                table: cache_t.name.clone(),
                column: cc.name.clone(),
                to: cc.default.clone(),
            });
        }
    }
}

fn diff_pk(cache_t: &Table, live_t: &Table, out: &mut Vec<Change>) {
    let cache_pk: Vec<String> = cache_t.columns.iter().filter(|c| c.primary_key).map(|c| c.name.clone()).collect();
    let live_pk:  Vec<String> = live_t.columns.iter().filter(|c| c.primary_key).map(|c| c.name.clone()).collect();
    match (cache_pk.is_empty(), live_pk.is_empty()) {
        (false, true) => out.push(Change::AddPrimaryKey { table: cache_t.name.clone(), columns: cache_pk }),
        (true, false) => out.push(Change::DropPrimaryKey { table: cache_t.name.clone() }),
        (false, false) if cache_pk != live_pk => {
            out.push(Change::DropPrimaryKey { table: cache_t.name.clone() });
            out.push(Change::AddPrimaryKey { table: cache_t.name.clone(), columns: cache_pk });
        }
        _ => {}
    }
}

fn diff_unique(cache_t: &Table, live_t: &Table, out: &mut Vec<Change>) {
    for cc in &cache_t.columns {
        let lc = live_t.columns.iter().find(|lc| lc.name == cc.name);
        match (cc.unique, lc.map(|lc| lc.unique)) {
            (true, Some(false)) | (true, None) => {
                out.push(Change::AddUnique { table: cache_t.name.clone(), columns: vec![cc.name.clone()] });
            }
            (false, Some(true)) => {
                out.push(Change::DropUnique { table: cache_t.name.clone(), columns: vec![cc.name.clone()] });
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 3: Write unit tests**

```rust
//! tests/diff_test.rs — extended in Task 11 for emit tests.

use sntl_migrate::diff::{compare, Change};
use sntl_schema::schema::{Column, PgTypeRef, Schema, Table};

fn col(name: &str, ty: &str, nullable: bool) -> Column {
    Column { name: name.into(), pg_type: PgTypeRef::simple(ty), oid: 0, nullable, primary_key: false, unique: false, default: None }
}

fn tbl(name: &str, cols: Vec<Column>) -> Table {
    Table { name: name.into(), schema: "public".into(), columns: cols, foreign_keys: vec![] }
}

fn sch(tables: Vec<Table>) -> Schema {
    Schema { version: 1, postgres_version: "16".into(), generated_at: None, source: None, tables, enums: vec![], composites: vec![] }
}

#[test]
fn add_table_change() {
    let cache = sch(vec![tbl("users", vec![col("id", "int4", false)])]);
    let live = sch(vec![]);
    let changes = compare(&cache, &live);
    assert!(matches!(changes[0], Change::AddTable(_)));
}

#[test]
fn drop_table_change() {
    let cache = sch(vec![]);
    let live = sch(vec![tbl("users", vec![col("id", "int4", false)])]);
    let changes = compare(&cache, &live);
    assert!(matches!(changes[0], Change::DropTable { ref name } if name == "users"));
}

#[test]
fn add_column_change() {
    let cache = sch(vec![tbl("u", vec![col("id", "int4", false), col("name", "text", false)])]);
    let live  = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let changes = compare(&cache, &live);
    assert!(changes.iter().any(|c| matches!(c, Change::AddColumn { table, .. } if table == "u")));
}

#[test]
fn alter_type_change() {
    let cache = sch(vec![tbl("u", vec![col("id", "int8", false)])]);
    let live  = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let changes = compare(&cache, &live);
    assert!(matches!(&changes[0], Change::AlterColumnType { from, to, .. } if from == "int4" && to == "int8"));
}
```

- [ ] **Step 4: Run, expect 4 passed**

- [ ] **Step 5: Commit**

```bash
git add sntl-migrate/src/diff/ sntl-migrate/tests/diff_test.rs
git commit -m "feat(sntl-migrate): diff::compare — Schema vs Schema → Vec<Change>"
```

---

### Task 11: `diff::emit` — Vec<Change> → SQL skeleton

**Files:**
- Create: `sntl-migrate/src/diff/emit.rs`
- Modify: `sntl-migrate/tests/diff_test.rs` (extend with emit tests)

- [ ] **Step 1: Implement `emit.rs`**

```rust
use super::compare::Change;

/// Generate a SQL skeleton from a list of changes plus a count of how many
/// emitted blocks need human review (TODO markers).
pub fn emit(changes: &[Change]) -> (String, usize) {
    let mut sql = String::new();
    let mut todos = 0usize;
    sql.push_str("-- Migration scaffold generated by `sntl migrate diff`\n");
    sql.push_str("-- Review TODO comments and remove the leading `-- ` to apply.\n\n");

    for c in changes {
        match c {
            Change::AddTable(t) => {
                sql.push_str(&format!("CREATE TABLE {} (\n", t.name));
                for (i, col) in t.columns.iter().enumerate() {
                    sql.push_str("    ");
                    sql.push_str(&col.name);
                    sql.push(' ');
                    sql.push_str(&col.pg_type.pg_type);
                    if !col.nullable { sql.push_str(" NOT NULL"); }
                    if let Some(d) = &col.default { sql.push_str(&format!(" DEFAULT {d}")); }
                    if i + 1 != t.columns.len() { sql.push(','); }
                    sql.push('\n');
                }
                let pk: Vec<&str> = t.columns.iter().filter(|c| c.primary_key).map(|c| c.name.as_str()).collect();
                if !pk.is_empty() {
                    sql.push_str(&format!("    , PRIMARY KEY ({})\n", pk.join(", ")));
                }
                sql.push_str(");\n\n");
            }
            Change::DropTable { name } => {
                todos += 1;
                sql.push_str("-- TODO: confirm DROP, destructive\n");
                sql.push_str(&format!("-- DROP TABLE {name} CASCADE;\n\n"));
            }
            Change::AddColumn { table, column } => {
                let has_default = column.default.is_some();
                let nullable = column.nullable;
                if has_default || nullable {
                    sql.push_str(&format!("ALTER TABLE {table} ADD COLUMN {} {}", column.name, column.pg_type.pg_type));
                    if !nullable { sql.push_str(" NOT NULL"); }
                    if let Some(d) = &column.default { sql.push_str(&format!(" DEFAULT {d}")); }
                    sql.push_str(";\n\n");
                } else {
                    todos += 1;
                    sql.push_str("-- TODO: NOT NULL without default — backfill required\n");
                    sql.push_str(&format!("-- ALTER TABLE {table} ADD COLUMN {} {} NOT NULL;\n\n", column.name, column.pg_type.pg_type));
                }
            }
            Change::DropColumn { table, column } => {
                todos += 1;
                sql.push_str("-- TODO: confirm DROP, destructive\n");
                sql.push_str(&format!("-- ALTER TABLE {table} DROP COLUMN {column};\n\n"));
            }
            Change::AlterColumnType { table, column, from, to } => {
                if is_widening(from, to) {
                    sql.push_str(&format!("ALTER TABLE {table} ALTER COLUMN {column} TYPE {to};\n\n"));
                } else {
                    todos += 1;
                    sql.push_str(&format!("-- TODO: cast may lose data ({from} → {to})\n"));
                    sql.push_str(&format!("-- ALTER TABLE {table} ALTER COLUMN {column} TYPE {to} USING {column}::{to};\n\n"));
                }
            }
            Change::AlterColumnNullable { table, column, to: true } => {
                sql.push_str(&format!("ALTER TABLE {table} ALTER COLUMN {column} DROP NOT NULL;\n\n"));
            }
            Change::AlterColumnNullable { table, column, to: false } => {
                todos += 1;
                sql.push_str("-- TODO: backfill NULLs first\n");
                sql.push_str(&format!("-- ALTER TABLE {table} ALTER COLUMN {column} SET NOT NULL;\n\n"));
            }
            Change::AlterColumnDefault { table, column, to: Some(d) } => {
                sql.push_str(&format!("ALTER TABLE {table} ALTER COLUMN {column} SET DEFAULT {d};\n\n"));
            }
            Change::AlterColumnDefault { table, column, to: None } => {
                sql.push_str(&format!("ALTER TABLE {table} ALTER COLUMN {column} DROP DEFAULT;\n\n"));
            }
            Change::AddPrimaryKey { table, columns } => {
                sql.push_str(&format!("ALTER TABLE {table} ADD PRIMARY KEY ({});\n\n", columns.join(", ")));
            }
            Change::DropPrimaryKey { table } => {
                todos += 1;
                sql.push_str("-- TODO: usually a structural change, review\n");
                sql.push_str(&format!("-- ALTER TABLE {table} DROP CONSTRAINT {table}_pkey;\n\n"));
            }
            Change::AddUnique { table, columns } => {
                let cs = columns.join("_");
                let cols = columns.join(", ");
                sql.push_str(&format!("CREATE UNIQUE INDEX {table}_{cs}_key ON {table} ({cols});\n\n"));
            }
            Change::DropUnique { table, columns } => {
                todos += 1;
                let cs = columns.join("_");
                sql.push_str("-- TODO: confirm drop of UNIQUE\n");
                sql.push_str(&format!("-- DROP INDEX {table}_{cs}_key;\n\n"));
            }
        }
    }

    (sql, todos)
}

fn is_widening(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("int2", "int4")
        | ("int2", "int8")
        | ("int4", "int8")
        | ("float4", "float8")
        | ("text", "text")
        | ("time", "timestamp")
        | ("time", "timetz")
        | ("timestamp", "timestamptz")
        | ("date", "timestamp")
        | ("date", "timestamptz")
    )
}
```

- [ ] **Step 2: Add emit tests to `diff_test.rs`**

Append to `sntl-migrate/tests/diff_test.rs`:

```rust
use sntl_migrate::diff::emit;

#[test]
fn emit_add_table_clean() {
    let cache = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let changes = compare(&cache, &sch(vec![]));
    let (out, todos) = emit(&changes);
    assert!(out.contains("CREATE TABLE u"));
    assert_eq!(todos, 0);
}

#[test]
fn emit_drop_table_is_todo() {
    let live = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let changes = compare(&sch(vec![]), &live);
    let (out, todos) = emit(&changes);
    assert_eq!(todos, 1);
    assert!(out.contains("TODO: confirm DROP"));
}

#[test]
fn emit_widening_alter_is_clean() {
    let cache = sch(vec![tbl("u", vec![col("id", "int8", false)])]);
    let live  = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let (out, todos) = emit(&compare(&cache, &live));
    assert_eq!(todos, 0);
    assert!(out.contains("ALTER TABLE u ALTER COLUMN id TYPE int8"));
}

#[test]
fn emit_narrowing_alter_has_todo() {
    let cache = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let live  = sch(vec![tbl("u", vec![col("id", "int8", false)])]);
    let (out, todos) = emit(&compare(&cache, &live));
    assert_eq!(todos, 1);
    assert!(out.contains("USING id::int4"));
}
```

- [ ] **Step 3: Run, expect all passed**

Run: `cargo test -p sntl-migrate --test diff_test`

- [ ] **Step 4: Commit**

```bash
git add sntl-migrate/src/diff/emit.rs sntl-migrate/tests/diff_test.rs
git commit -m "feat(sntl-migrate): diff::emit — SQL skeleton with TODO annotations"
```

---

### Task 12: `macro_support` + `sntl-macros::migrate!()` proc-macro

**Files:**
- Modify: `sntl-migrate/src/macro_support.rs`
- Create: `sntl-macros/src/migrate/mod.rs`
- Create: `sntl-macros/src/migrate/codegen.rs`
- Modify: `sntl-macros/src/lib.rs` (register macro)
- Modify: `sntl-macros/Cargo.toml` (add sntl-migrate dev-dep + walkdir)

- [ ] **Step 1: `macro_support.rs` is just a re-export shim**

```rust
//! Helpers consumed by the `sntl_migrate::migrate!()` proc-macro.
//!
//! These exist so the macro's generated code has a stable path that does
//! not require it to know about internal `Migrator` constructors.

pub use crate::migration::TxMode;
pub use crate::runner::Migrator;
```

- [ ] **Step 2: Implement `sntl-macros/src/migrate/codegen.rs`**

```rust
use proc_macro2::TokenStream;
use proc_macro_error2::abort;
use quote::quote;
use std::path::{Path, PathBuf};
use syn::LitStr;

pub fn expand(input: TokenStream) -> TokenStream {
    let lit: LitStr = match syn::parse2(input) {
        Ok(l) => l,
        Err(e) => abort!(e.span(), "{}", e),
    };
    let rel = lit.value();
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let migrations_dir = manifest.join(&rel);
    if !migrations_dir.is_dir() {
        abort!(lit.span(), "migrations directory not found: {}", migrations_dir.display());
    }

    let mut entries = Vec::new();
    let read = match std::fs::read_dir(&migrations_dir) {
        Ok(r) => r,
        Err(e) => abort!(lit.span(), "read_dir({}): {e}", migrations_dir.display()),
    };
    for entry in read.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
        let name = entry.file_name().to_string_lossy().into_owned();
        let dir = entry.path();
        let (sql_path, is_notx) = if dir.join("up.notx.sql").exists() {
            (dir.join("up.notx.sql"), true)
        } else if dir.join("up.sql").exists() {
            (dir.join("up.sql"), false)
        } else {
            abort!(lit.span(), "migration `{name}` has neither up.sql nor up.notx.sql");
        };
        entries.push((name, sql_path, is_notx));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let tokens: Vec<TokenStream> = entries.iter().map(|(name, path, is_notx)| {
        let path_str = path.to_string_lossy().into_owned();
        let mode = if *is_notx {
            quote! { ::sntl_migrate::TxMode::None }
        } else {
            quote! { ::sntl_migrate::TxMode::PerMigration }
        };
        quote! {
            (#name, include_str!(#path_str), #mode),
        }
    }).collect();

    quote! {
        ::sntl_migrate::Migrator::from_static(&[
            #(#tokens)*
        ])
    }
}
```

- [ ] **Step 3: `sntl-macros/src/migrate/mod.rs`**

```rust
mod codegen;
pub use codegen::expand;
```

- [ ] **Step 4: Register macro in `sntl-macros/src/lib.rs`**

Add:

```rust
mod migrate;

#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn migrate(input: TokenStream) -> TokenStream {
    migrate::expand(input.into()).into()
}
```

- [ ] **Step 5: Re-export from `sntl-migrate/src/lib.rs`**

Add: `pub use sntl_macros::migrate;`

- [ ] **Step 6: Build + commit**

```bash
cargo check --workspace
git add sntl-migrate/src/{macro_support,lib}.rs sntl-macros/src/{lib,migrate}.rs sntl-macros/src/migrate/codegen.rs
git commit -m "feat(sntl-macros): migrate!() proc-macro for compile-time migration bundling"
```

---

### Task 13: Trybuild fixture for `migrate!()`

**Files:**
- Create: `sntl-migrate/tests/embedded_test.rs`
- Create: `sntl-migrate/tests/embedded_fixtures/migrations/20260101_000000_seed/up.sql`

- [ ] **Step 1: Seed a tiny fixture**

```sql
-- sntl-migrate/tests/embedded_fixtures/migrations/20260101_000000_seed/up.sql
CREATE TABLE embedded_test_t (id int);
```

- [ ] **Step 2: Smoke test**

```rust
#[test]
fn embedded_macro_compiles() {
    let _migrator = sntl_migrate::migrate!("./tests/embedded_fixtures/migrations");
    // not run — just verify the macro expands and types check
}
```

- [ ] **Step 3: Run + commit**

Run: `cargo test -p sntl-migrate --test embedded_test`
Expected: 1 passed.

```bash
git add sntl-migrate/tests/embedded_test.rs sntl-migrate/tests/embedded_fixtures/
git commit -m "test(sntl-migrate): trybuild fixture verifying migrate!() macro"
```

---

### Task 14: `sntl-cli migrate add`

**Files:**
- Create: `sntl-cli/src/commands/migrate.rs`
- Modify: `sntl-cli/src/commands/mod.rs` (`pub mod migrate;`)
- Modify: `sntl-cli/src/main.rs` (register subcommand)
- Modify: `sntl-cli/Cargo.toml` (add sntl-migrate dep)

- [ ] **Step 1: Cargo.toml addition**

Add `sntl-migrate.workspace = true` to `sntl-cli/Cargo.toml` `[dependencies]`.

- [ ] **Step 2: Subcommand surface in `main.rs`**

Extend the `Command` enum:

```rust
/// Manage SQL migrations
Migrate {
    #[command(subcommand)]
    action: MigrateCmd,
},
```

```rust
#[derive(clap::Subcommand)]
enum MigrateCmd {
    /// Scaffold a new migration folder
    Add { name: String, #[arg(long)] no_create_dir: bool },
    /// Apply pending migrations
    Run { #[arg(long)] dry_run: bool, #[arg(long)] skip_refresh: bool },
    /// Show applied + pending
    Info { #[arg(long)] applied: bool, #[arg(long)] pending: bool, #[arg(long)] all: bool },
    /// Compare cache vs DB, emit SQL scaffold
    Diff { #[arg(long)] out: Option<String> },
    /// Verify applied migrations match their files
    Verify,
}
```

In `main`, route `Migrate { action }` to `commands::migrate::dispatch(...)`.

- [ ] **Step 3: Implement `add` in `commands/migrate.rs`**

```rust
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

use crate::ui;

pub async fn add(workspace: Option<PathBuf>, name: String, no_create_dir: bool) -> Result<()> {
    let root = workspace.unwrap_or_else(|| std::env::current_dir().unwrap());
    let migrations = root.join("migrations");
    if !migrations.exists() {
        if no_create_dir {
            return Err(anyhow!("migrations/ does not exist (use without --no-create-dir to create)"));
        }
        std::fs::create_dir_all(&migrations).context("create migrations/")?;
    }
    let sanitized = sanitize_name(&name)?;
    let ts = utc_now_compact();
    let folder = migrations.join(format!("{ts}_{sanitized}"));
    std::fs::create_dir_all(&folder).context("create migration folder")?;
    let up = folder.join("up.sql");
    std::fs::write(&up, header_template(&ts, &sanitized)).context("write up.sql")?;
    ui::ok(&format!("created {}", up.display()));
    println!("ℹ edit it, then run `sntl migrate run`");
    Ok(())
}

fn sanitize_name(name: &str) -> Result<String> {
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
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    // YYYYMMDD_HHMMSS — derive via chrono
    let dt = chrono::DateTime::<chrono::Utc>::from(std::time::UNIX_EPOCH + now);
    dt.format("%Y%m%d_%H%M%S").to_string()
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
```

Plus a `dispatch` fn at the top:

```rust
pub async fn dispatch(
    workspace: Option<PathBuf>,
    database_url: Option<String>,
    action: crate::MigrateCmd,
) -> Result<()> {
    use crate::MigrateCmd::*;
    match action {
        Add { name, no_create_dir } => add(workspace, name, no_create_dir).await,
        Run { dry_run, skip_refresh } => run(workspace, database_url, dry_run, skip_refresh).await,
        Info { applied, pending, all } => info(workspace, database_url, applied, pending, all).await,
        Diff { out } => diff(workspace, database_url, out).await,
        Verify => verify(workspace, database_url).await,
    }
}
```

Stub the other handlers to `anyhow::bail!("not yet implemented")` for now — Tasks 15–17 fill them in.

- [ ] **Step 4: Wire `mod.rs` + `chrono` dep**

In `sntl-cli/src/commands/mod.rs` add `pub mod migrate;`.
In `sntl-cli/Cargo.toml` `[dependencies]` add `chrono.workspace = true`.

- [ ] **Step 5: Quick integration test**

Create `sntl-cli/tests/migrate_add_test.rs`:

```rust
use std::path::Path;

fn cli_binary() -> &'static str { env!("CARGO_BIN_EXE_sntl") }

#[test]
fn add_creates_folder_and_up_sql() {
    let dir = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(cli_binary())
        .arg("--workspace").arg(dir.path()).arg("migrate").arg("add").arg("add users")
        .output().unwrap();
    assert!(out.status.success(), "stderr={:?}", String::from_utf8_lossy(&out.stderr));
    let mig_dir = dir.path().join("migrations");
    assert!(mig_dir.is_dir());
    let inside: Vec<_> = std::fs::read_dir(&mig_dir).unwrap().flatten().collect();
    assert_eq!(inside.len(), 1);
    let folder_name = inside[0].file_name().to_string_lossy().into_owned();
    assert!(folder_name.ends_with("_add_users"), "got {folder_name}");
    assert!(Path::new(&inside[0].path().join("up.sql")).exists());
}
```

- [ ] **Step 6: Build + test + commit**

```bash
cargo test -p sntl-cli --test migrate_add_test
git add sntl-cli/ 
git commit -m "feat(sntl-cli): sntl migrate add — scaffold new migration folder"
```

---

### Task 15: `sntl-cli migrate run` + `info` + `verify`

**Files:**
- Modify: `sntl-cli/src/commands/migrate.rs`
- Create: `sntl-cli/tests/migrate_run_test.rs`

- [ ] **Step 1: Implement `run` / `info` / `verify` in `migrate.rs`**

```rust
use sntl_migrate::{Migrator, runner::State};
use sntl_schema::config::Config;

pub async fn run(
    workspace: Option<PathBuf>,
    database_url: Option<String>,
    dry_run: bool,
    skip_refresh: bool,
) -> Result<()> {
    let (root, url) = resolve(workspace, database_url)?;
    let migrations = root.join("migrations");
    let migrator = Migrator::from_dir(&migrations)
        .with_context(|| format!("discover {}", migrations.display()))?;

    if dry_run {
        ui::ok("dry-run — would apply:");
        for m in migrator.migrations() {
            println!("  ◯ {}", m.version);
        }
        return Ok(());
    }

    let cfg_pool = sentinel_driver::pool::config::PoolConfig::new().max_connections(4);
    let driver_cfg = sentinel_driver::Config::parse(&url).context("parse DATABASE_URL")?;
    let pool = sentinel_driver::Pool::new(driver_cfg, cfg_pool);

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
        ui::ok(&format!("{} migrations applied", report.applied.len()));
    }
    if !skip_refresh {
        ui::ok("refreshed .sentinel/schema.toml");
    }
    Ok(())
}

pub async fn info(
    workspace: Option<PathBuf>,
    database_url: Option<String>,
    show_applied: bool,
    show_pending: bool,
    show_all: bool,
) -> Result<()> {
    let (root, url) = resolve(workspace, database_url)?;
    let migrations = root.join("migrations");
    let migrator = Migrator::from_dir(&migrations)?;
    let pool = pool_for(&url)?;
    let statuses = migrator.info(&pool).await?;
    let want_applied = show_applied || show_all || (!show_applied && !show_pending);
    let want_pending = show_pending || show_all || (!show_applied && !show_pending);
    for s in statuses {
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

pub async fn verify(workspace: Option<PathBuf>, database_url: Option<String>) -> Result<()> {
    let (root, url) = resolve(workspace, database_url)?;
    let migrations = root.join("migrations");
    let migrator = Migrator::from_dir(&migrations)?;
    let pool = pool_for(&url)?;
    let statuses = migrator.info(&pool).await?;
    let drifted: Vec<_> = statuses.iter().filter(|s| s.state == State::ChecksumDrift).collect();
    if drifted.is_empty() {
        ui::ok(&format!("all {} applied migrations have matching checksums", statuses.iter().filter(|s| s.state == State::Applied).count()));
        Ok(())
    } else {
        for d in &drifted {
            ui::warn(&format!("checksum drift in {}", d.version));
        }
        Err(anyhow!("verify failed"))
    }
}

fn resolve(workspace: Option<PathBuf>, database_url: Option<String>) -> Result<(PathBuf, String)> {
    let root = workspace.unwrap_or_else(|| std::env::current_dir().unwrap());
    let mut cfg = Config::load_from(root.join("sentinel.toml")).unwrap_or_default();
    if let Some(u) = database_url { cfg.database.url = Some(u); }
    let url = cfg.database.url.ok_or_else(|| anyhow!("database_url not configured"))?;
    Ok((root, url))
}

fn pool_for(url: &str) -> Result<sentinel_driver::Pool> {
    let cfg = sentinel_driver::Config::parse(url).context("parse DATABASE_URL")?;
    Ok(sentinel_driver::Pool::new(cfg, sentinel_driver::pool::config::PoolConfig::new().max_connections(4)))
}
```

- [ ] **Step 2: End-to-end integration test (live PG)**

```rust
//! tests/migrate_run_test.rs
use std::process::Command;

fn cli() -> &'static str { env!("CARGO_BIN_EXE_sntl") }

#[test]
fn add_then_run_then_info() {
    let url = match std::env::var("DATABASE_URL").ok() { Some(u) => u, None => return };
    let dir = tempfile::tempdir().unwrap();

    // Clean prior state
    let mut admin = std::process::Command::new("psql").arg(&url).arg("-c").arg("DROP TABLE IF EXISTS _sntl_migrations, e2e_test").output().unwrap();
    assert!(admin.status.success());

    // add
    Command::new(cli()).args(["--workspace", &dir.path().to_string_lossy(), "migrate", "add", "create e2e"]).status().unwrap();
    // Replace up.sql with real SQL
    let folder = std::fs::read_dir(dir.path().join("migrations")).unwrap().next().unwrap().unwrap();
    std::fs::write(folder.path().join("up.sql"), "CREATE TABLE e2e_test (id int);").unwrap();

    // run
    let out = Command::new(cli()).args(["--workspace", &dir.path().to_string_lossy(), "--database-url", &url, "migrate", "run"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    // info shows it applied
    let out = Command::new(cli()).args(["--workspace", &dir.path().to_string_lossy(), "--database-url", &url, "migrate", "info", "--all"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("✓"), "stdout was: {stdout}");
}
```

- [ ] **Step 3: Build + run + commit**

```bash
DATABASE_URL=… cargo test -p sntl-cli --test migrate_run_test
git add sntl-cli/
git commit -m "feat(sntl-cli): migrate run/info/verify subcommands"
```

---

### Task 16: `sntl-cli migrate diff`

**Files:**
- Modify: `sntl-cli/src/commands/migrate.rs`
- Create: `sntl-cli/tests/migrate_diff_test.rs`

- [ ] **Step 1: Implement `diff` in `migrate.rs`**

```rust
pub async fn diff(workspace: Option<PathBuf>, database_url: Option<String>, out: Option<String>) -> Result<()> {
    let (root, url) = resolve(workspace, database_url)?;
    let cache_path = root.join(".sentinel/schema.toml");
    let cache_text = std::fs::read_to_string(&cache_path)
        .with_context(|| format!("read {}", cache_path.display()))?;
    let cache: sntl_schema::schema::Schema = toml::from_str(&cache_text)
        .context("parse .sentinel/schema.toml")?;
    let live = sntl_schema::introspect::pull_schema(&url).await
        .context("pull live schema")?;

    let changes = sntl_migrate::diff::compare(&cache, &live);
    if changes.is_empty() {
        ui::ok("no differences");
        return Ok(());
    }
    let (sql, todos) = sntl_migrate::diff::emit(&changes);

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let suffix = out.unwrap_or_else(|| "diff".to_string());
    let folder = root.join("migrations").join(format!("{ts}_{suffix}"));
    std::fs::create_dir_all(&folder)?;
    let up = folder.join("up.sql");
    std::fs::write(&up, sql)?;

    ui::ok(&format!("wrote {}", up.display()));
    println!("ℹ {} changes ({} TODO)", changes.len(), todos);
    Ok(())
}
```

- [ ] **Step 2: Integration test**

```rust
//! tests/migrate_diff_test.rs
use std::process::Command;

fn cli() -> &'static str { env!("CARGO_BIN_EXE_sntl") }

#[test]
fn diff_emits_file_when_drift_exists() {
    let url = match std::env::var("DATABASE_URL").ok() { Some(u) => u, None => return };
    let dir = tempfile::tempdir().unwrap();

    // Seed a fake cache schema that has a table the DB doesn't.
    std::fs::create_dir_all(dir.path().join(".sentinel/queries")).unwrap();
    std::fs::write(dir.path().join(".sentinel/.version"), "1").unwrap();
    std::fs::write(dir.path().join(".sentinel/schema.toml"), r#"
version = 1
postgres_version = "16"

[[tables]]
name = "fake_diff_table"
schema = "public"

  [[tables.columns]]
  name = "id"
  pg_type = "int4"
  oid = 23
  nullable = false
  primary_key = true
"#).unwrap();

    let out = Command::new(cli()).args(["--workspace", &dir.path().to_string_lossy(), "--database-url", &url, "migrate", "diff"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let migrations = std::fs::read_dir(dir.path().join("migrations")).unwrap();
    let count = migrations.count();
    assert_eq!(count, 1);
}
```

- [ ] **Step 3: Build + test + commit**

```bash
DATABASE_URL=… cargo test -p sntl-cli --test migrate_diff_test
git add sntl-cli/
git commit -m "feat(sntl-cli): migrate diff — emit SQL scaffold from cache vs DB"
```

---

### Task 17: PR-2 wrap-up

- [ ] Workspace verification (same suite as Task 9)
- [ ] Push + open PR-2

```bash
git push
gh pr create --title "feat(sntl-migrate): diff + macro + CLI subcommands" --body "..."
```

---

## Phase 3 — Polish + docs (PR-3)

### Task 18: Concurrent-apply integration test

**Files:**
- Modify: `sntl-migrate/tests/runner_test.rs` (add a `lock_serialises_two_runners` test)

- [ ] **Step 1: Append test**

```rust
#[tokio::test]
async fn lock_serialises_two_runners() {
    let Some(pool) = pool().await else { return };
    let mut admin = pool.acquire().await.unwrap();
    admin.execute("DROP TABLE IF EXISTS _sntl_migrations, lock_test", &[]).await.unwrap();
    drop(admin);

    let dir = tempdir().unwrap();
    write_mig(dir.path(), "20260509_140000_lock", "CREATE TABLE lock_test (id int); SELECT pg_sleep(1);");
    let path = dir.path().join("migrations");

    let m1 = Migrator::from_dir(&path).unwrap();
    let m2 = Migrator::from_dir(&path).unwrap();
    let p1 = pool.clone();
    let p2 = pool.clone();

    let h1 = tokio::spawn(async move { m1.run(&p1).await });
    // small delay so h1 acquires lock first
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let h2 = tokio::spawn(async move { m2.run(&p2).await });

    let (r1, r2) = tokio::join!(h1, h2);
    r1.unwrap().unwrap();
    r2.unwrap().unwrap();
    // lock_test exists exactly once — concurrent run did not double-apply.
}
```

- [ ] **Step 2: Run + commit**

Run: `DATABASE_URL=… cargo test -p sntl-migrate --test runner_test lock_serialises -- --test-threads=1`

```bash
git add sntl-migrate/tests/runner_test.rs
git commit -m "test(sntl-migrate): concurrent run serialised by advisory lock"
```

---

### Task 19: `docs/migration-guide.md`

**Files:**
- Create: `docs/migration-guide.md`

- [ ] **Step 1: Write user guide**

Cover:
- Quick start: `sntl init && sntl migrate add foo`
- Daily workflow: edit `up.sql` → `sntl migrate run` (auto refreshes cache)
- Production embed: `sntl_migrate::migrate!("./migrations").run(&pool).await?`
- Non-transactional DDL → `up.notx.sql`
- Strict ordering rule + how to rebase your branch when out of order
- Diff workflow: `sntl migrate diff` → edit TODOs → `sntl migrate run`
- Concurrent deploys: how advisory lock serialises
- CI integration: `sntl migrate verify` as deploy guard
- Limitations (no down/revert in v0.3, no FK diff, no enum/composite diff)

- [ ] **Step 2: Commit**

```bash
git add docs/migration-guide.md
git commit -m "docs: sntl-migrate user guide"
```

---

### Task 20: README + roadmap updates

**Files:**
- Modify: `README.md` (architecture section: sntl-migrate moved from `(planned)` to shipping)
- Update memory `roadmap_sentinel.md`: mark migration as shipped, advance v0.3 progress.

- [ ] **Step 1: Update README**

In `README.md`'s architecture box, change `sntl-migrate # Schema diff & migration generation (planned)` to indicate it ships in v0.3 with a link to `docs/migration-guide.md`.

- [ ] **Step 2: Update memory roadmap**

Edit `~/.claude/projects/.../memory/roadmap_sentinel.md` v0.3 section to check off `sntl-migrate`.

- [ ] **Step 3: Commit + push PR-3**

```bash
git add README.md
git commit -m "docs: README architecture + link to migration guide"
git push
gh pr create --title "docs(sntl-migrate): user guide + README polish" --body "..."
```

---

## Self-Review

**Spec coverage** (against `docs/plans/2026-05-09-sntl-migrate-design.md`):

| Spec section | Implementing task(s) |
|---|---|
| §1 Migrator library API | Tasks 3, 5, 7, 8 |
| §1 CLI subcommands | Tasks 14, 15, 16 |
| §1 compile-time macro | Tasks 12, 13 |
| §1 diff scaffolder | Tasks 10, 11, 16 |
| §1 auto-refresh schema.toml | Task 8 |
| §2 file structure | Task 1 |
| §3 `sntl migrate add` | Task 14 |
| §3 `sntl migrate run` | Task 15 |
| §3 `sntl migrate info` | Task 15 |
| §3 `sntl migrate diff` | Task 16 |
| §3 `sntl migrate verify` | Task 15 |
| §4 Change enum | Task 10 |
| §4 emit + widening table | Task 11 |
| §4 no rename detection | Task 11 (implicit — no rename code) |
| §5 error UX | Task 2 (struct) + Task 15 (UI surface via ui::err) |
| §5 testing strategy | Tasks 3-7 (unit), 6-7-15-16-18 (live PG), 13 (macro) |

**Placeholder scan:** none.

**Type consistency:**
- `Version`, `Migration`, `TxMode`, `MigrationReport`, `MigrationStatus`, `State`, `Migrator` — defined Tasks 3 + 7, consumed across all later tasks consistently.
- `RefreshConfig` introduced Task 8, used internally only.
- Macro emits `Migrator::from_static(&[(name, sql, TxMode)])` — matches the signature defined in Task 7.

**Open items** — none after the API spike at the top of this plan. Implementer can proceed.

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-05-09-sntl-migrate-impl.md`. Two execution options:

**1. Subagent-Driven** — fresh subagent per task. Costly per the prior Cluster A experience (Task 5 burned 1M tokens) but rigorous review.

**2. Inline Execution** — run tasks in this session with checkpoints after each phase. Faster, matches the cadence that worked well for PR #12 and PR #13.

Recommend **inline execution** given the cost data from the PR #14 cycle. Which approach?
