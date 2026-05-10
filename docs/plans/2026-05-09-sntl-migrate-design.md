# `sntl-migrate` v0.3 — Design

> **Status:** approved by user, ready for implementation plan
> **Goal:** First-class migration tooling closing the largest user-facing parity gap with sqlx/SeaORM/Diesel. Forward-only MVP plus the unique-to-Sentinel `sntl migrate diff` (cache vs DB scaffolder).

## 1. Goals + scope

### What ships in v0.3

1. **Library crate `sntl-migrate`** — `Migrator` type with `from_dir()` and `from_static()` constructors and `run()` / `info()` / `applied_versions()` methods.
2. **CLI subcommands in `sntl-cli`** — `sntl migrate add` / `run` / `info` / `diff` / `verify`.
3. **Compile-time bundling macro** — `sntl_migrate::migrate!("./migrations")` for embedded production deployment.
4. **Schema-diff scaffolder** — `sntl migrate diff` compares `.sentinel/schema.toml` against the live DB and emits a skeleton `up.sql` with `-- TODO:` comments for ambiguous changes.
5. **Auto-refresh `.sentinel/schema.toml`** — every `sntl migrate run` re-pulls schema after apply so the macro cache never goes stale.

### Out of scope (v0.4+)

- `down.sql` / revert (folder layout reserves room but no implementation)
- Lax out-of-order mode (strict only in v0.3)
- Sequential / incremental tracking modes
- Database engines other than PostgreSQL
- User-defined composite-type diff (basic types only)
- Schema introspection beyond what `pull_schema` already covers
- Migration squash / consolidation
- Branch-aware migrations
- Two-phase apply (apply, verify, commit)
- Indexes other than PRIMARY KEY / UNIQUE
- Triggers, views, functions

### Success criteria

| # | Statement |
|---|-----------|
| 1 | `sntl init && sntl migrate add foo` produces a working `migrations/<ts>_foo/up.sql`. |
| 2 | `sntl migrate run` against a fresh DB applies all pending in timestamp order and creates `_sntl_migrations`. |
| 3 | `sntl migrate run` on already-applied schema is a no-op and exits 0. |
| 4 | Concurrent `sntl migrate run` from multiple processes serialises via PostgreSQL advisory lock; only one applies. |
| 5 | After `sntl migrate run`, `.sentinel/schema.toml` reflects the new schema automatically. |
| 6 | `sntl migrate info` shows applied/pending lists with timestamps and checksums. |
| 7 | `sntl migrate diff` against a drifted DB emits valid SQL skeleton with TODOs flagging destructive cases. |
| 8 | An out-of-order migration produces a clear error with rename instructions. |
| 9 | `up.notx.sql` runs without a transaction wrapper. |
| 10 | `sntl_migrate::migrate!("./migrations").run(&pool).await?` works from a production binary with no external SQL files. |
| 11 | An applied migration whose `up.sql` was later modified surfaces as a checksum drift warning (not error). |

## 2. Architecture + crate boundaries

### File structure (master map)

#### New library crate: `sntl-migrate/`

```
sntl-migrate/
├── Cargo.toml                    # NEW (replaces existing stub)
├── src/
│   ├── lib.rs                    # public surface — Migrator, Error, MigrationReport
│   ├── error.rs                  # typed errors (thiserror)
│   ├── migration.rs              # Migration struct + Version newtype + parsing
│   ├── discover.rs               # filesystem walk → Vec<Migration>
│   ├── tracking.rs               # _sntl_migrations table CREATE/SELECT/INSERT
│   ├── runner.rs                 # apply_one + run loop + advisory lock
│   ├── checksum.rs               # sha256 of up.sql contents
│   ├── refresh.rs                # post-run hook: pull_schema → write schema.toml
│   ├── diff/
│   │   ├── mod.rs                # entry point
│   │   ├── compare.rs            # Schema vs Schema → Vec<Change>
│   │   └── emit.rs               # Vec<Change> → SQL skeleton with TODOs
│   └── macro_support.rs          # runtime helpers for migrate!() macro
└── tests/
    ├── discover_test.rs
    ├── tracking_test.rs
    ├── runner_test.rs
    ├── diff_test.rs
    └── embedded_test.rs
```

#### Extended crate: `sntl-macros/`

```
sntl-macros/src/
├── lib.rs                        # MODIFY: register migrate! proc-macro
└── migrate/
    ├── mod.rs                    # NEW
    └── codegen.rs                # NEW — emit Migrator::from_static(...)
```

#### Extended crate: `sntl-cli/`

```
sntl-cli/src/commands/
├── mod.rs                        # MODIFY: pub mod migrate
└── migrate.rs                    # NEW — clap subcommand handlers (add/run/info/diff/verify)
```

### Module responsibilities

| Module | One-line responsibility |
|---|---|
| `migration.rs` | `Migration { version, name, sql, tx_mode }` + `Version` ord newtype |
| `discover.rs` | Walks `migrations/` → returns `Vec<Migration>` sorted by `Version` |
| `tracking.rs` | Owns `_sntl_migrations`: `ensure()`, `applied_versions()`, `record(version, checksum)` |
| `runner.rs` | `Migrator::run` — acquire lock, validate ordering, apply each, record |
| `checksum.rs` | `sha256_of_sql(content) -> String` (8 lines, no cross-crate dep) |
| `refresh.rs` | Post-apply hook: `sntl_schema::introspect::pull_schema` → write `.sentinel/schema.toml` |
| `diff/compare.rs` | `(Schema, Schema) -> Vec<Change>` (Add/Drop/Alter Table/Column) |
| `diff/emit.rs` | `Vec<Change> -> (String, todo_count)` SQL skeleton |
| `macro_support.rs` | `Migrator::from_static(&[(name, sql, tx_mode), ...])` |

### Crate dependency graph

```
sntl-migrate (lib)
    ├── sentinel-driver       (Connection, Pool, advisory lock, query, execute)
    ├── sntl-schema           (Schema struct + introspect::pull_schema)
    ├── sha2 + hex            (checksum)
    ├── thiserror             (typed errors)
    ├── tokio                 (async)
    └── walkdir               (folder discovery)

sntl-macros (extend)
    └── sntl-migrate          (NEW dep — emitted code references Migrator types)

sntl-cli (extend)
    └── sntl-migrate          (NEW dep — wraps Migrator for CLI subcommands)

sntl (no change)
    # sntl-migrate is NOT a dep of sntl. Users opt in via cargo add sntl-migrate.
```

### Public API surface (sntl-migrate v0.1.0)

```rust
pub use error::{Error, Result};
pub use migration::{Migration, Version, TxMode};
pub use runner::{Migrator, MigrationReport, MigrationStatus};

// Re-export macro from sntl-macros for ergonomics:
pub use sntl_macros::migrate;
```

`Migrator` public methods:

- `from_dir(path: impl AsRef<Path>) -> Result<Self>`
- `from_static(entries: &'static [(/* version */ &str, /* sql */ &str, TxMode)]) -> Self`
- `run(&self, pool: &Pool) -> Result<MigrationReport>`
- `info(&self, pool: &Pool) -> Result<Vec<MigrationStatus>>`
- `applied_versions(&self, pool: &Pool) -> Result<Vec<Version>>`

## 3. CLI command surface

### Subcommand grammar

```
sntl migrate add <name>           [--no-create-dir]
sntl migrate run                  [--dry-run] [--skip-refresh]
sntl migrate info                 [--applied|--pending|--all]
sntl migrate diff                 [--out <path>]
sntl migrate verify
```

Global flags inherited from `sntl`: `--workspace <path>`, `--database-url <url>`.

### `sntl migrate add <name>`

```bash
$ sntl migrate add add_users
✓ created migrations/20260509_140000_add_users/up.sql
ℹ edit it, then run `sntl migrate run`
```

Behavior:

- Sanitize `<name>` → snake_case (regex `[^a-z0-9_]` → `_`, collapse repeats).
- Generate UTC timestamp `YYYYMMDD_HHMMSS`.
- Create `migrations/<ts>_<sanitized>/` (mkdir parent if missing — unless `--no-create-dir`).
- Write `up.sql` with template header.
- Print path.

`up.sql` template:

```sql
-- Migration: <ts>_<name>
-- Created: <ts UTC>
--
-- This file runs in a single PostgreSQL transaction. Rename to
-- `up.notx.sql` if you need non-transactional DDL (CREATE INDEX
-- CONCURRENTLY, REFRESH MATERIALIZED VIEW CONCURRENTLY, etc.).
```

### `sntl migrate run`

```bash
$ sntl migrate run
✓ acquired migration lock
ℹ found 3 pending migrations
✓ applied 20260507_140000_add_users        (12 ms)
✓ applied 20260508_090000_add_posts        (8 ms)
✓ applied 20260509_120000_index_posts      (43 ms)  [no-tx]
✓ refreshed .sentinel/schema.toml
✓ all migrations applied
```

Behavior:

1. Connect via `DATABASE_URL` (or `sentinel.toml` `[database] url`).
2. Acquire `pg_advisory_lock(SNTL_MIGRATE_LOCK)`.
3. `tracking::ensure()` — `CREATE TABLE IF NOT EXISTS _sntl_migrations`.
4. Load applied versions.
5. Discover pending from `migrations/`.
6. **Strict order check** — error if any pending precedes max applied.
7. For each pending in order:
   - Read `up.sql` (or `up.notx.sql` → `TxMode::None`).
   - If `TxMode::PerMigration`: `BEGIN; <sql>; COMMIT`. Else: raw exec.
   - On success: `INSERT INTO _sntl_migrations (version, checksum, applied_at)`.
   - On failure: rollback (transactional), exit 1, error identifies file + line.
8. After all: `refresh::pull_and_write()` — auto-update `.sentinel/schema.toml`.
9. Lock auto-released on connection close.

Flags:

- `--dry-run` — print what would run, no changes.
- `--skip-refresh` — apply migrations but do not refresh `schema.toml` (CI use case).

Exit codes:

- 0: success (incl. no-op when nothing pending).
- 1: any error.

### `sntl migrate info`

```bash
$ sntl migrate info
ℹ migrations directory: migrations/
ℹ tracking table: _sntl_migrations (5 applied, 2 pending)

Applied:
  ✓ 20260101_000000_initial             a3f7c2e9b1d4a  applied 2026-04-15T10:30:00Z
  ✓ 20260415_140000_add_users           7b076c5336a05  applied 2026-04-20T08:15:00Z
  ✓ 20260420_080000_add_posts           ea592aaabe154  applied 2026-04-22T14:00:00Z
  ⚠ 20260501_120000_index_users         93a2f647a0777  applied 2026-05-01T13:00:00Z  (file modified after apply)
  ✓ 20260505_090000_add_tags            2626d3f4e5d1c  applied 2026-05-05T09:30:00Z

Pending:
  ◯ 20260508_140000_alter_email
  ◯ 20260509_100000_add_index_email
```

Flags `--applied` / `--pending` / `--all` filter output.

### `sntl migrate diff`

```bash
$ sntl migrate diff
ℹ comparing live DB vs .sentinel/schema.toml
ℹ found 3 differences (2 clean, 1 needs review)
✓ wrote migrations/20260509_140000_diff/up.sql

Review the TODO comments before running `sntl migrate run`:
  - users.middle_name: type narrowing (varchar→text→varchar(100))
```

Behavior:

1. Load `.sentinel/schema.toml`.
2. Connect to DB → `pull_schema` → in-memory `Schema`.
3. `diff::compare` → `Vec<Change>`.
4. `diff::emit` → SQL string + TODO count.
5. Write to next-timestamp `migrations/<ts>_diff/up.sql`.
6. Print summary + flag review-needed cases.

Default name `_diff` overridable via `--out <name>`.

### `sntl migrate verify`

```bash
$ sntl migrate verify
✓ all 5 applied migrations have matching checksums
```

Or:

```bash
$ sntl migrate verify
⚠ checksum drift in 1 migration:
  - 20260501_120000_index_users
    file:     93a2f647a0777
    recorded: a8c3d2f1e90b4
✗ verify failed
```

Exits 1 on any mismatch — CI-friendly guard before deploy.

### UX style

- ✓/⚠/✗/◯/ℹ prefix via `sntl-cli`'s `ui` module (already on owo-colors).
- All output to stdout; errors to stderr.
- `RUST_LOG=debug` enables timing breakdowns + SQL trace per migration.

## 4. Diff algorithm

### Input → output flow

```
.sentinel/schema.toml ──┐
                        ├──> diff::compare ──> Vec<Change>
DB (via pull_schema) ───┘                          │
                                                   ▼
                                         diff::emit ──> SQL skeleton
                                                        + TODO count
```

### `Change` enum

```rust
enum Change {
    AddTable(Table),
    DropTable { name: String },
    AddColumn  { table: String, column: Column },
    DropColumn { table: String, column: String },
    AlterColumnType { table: String, column: String, from: String, to: String },
    AlterColumnNullable { table: String, column: String, to: bool },
    AlterColumnDefault { table: String, column: String, to: Option<String> },
    AddPrimaryKey   { table: String, columns: Vec<String> },
    DropPrimaryKey  { table: String },
    AddUnique       { table: String, columns: Vec<String> },
    DropUnique      { table: String, columns: Vec<String> },
    AddForeignKey   { table: String, fk: ForeignKey },
    DropForeignKey  { table: String, name: String },
}
```

### Out of v0.3 diff scope

- Indexes other than PK/UNIQUE (`Schema` struct does not track them yet)
- Enum and composite types (sub-project #4 in roadmap)
- Triggers, views, functions
- Column ordering changes
- Comments / DOC strings on columns

### Emit rules per `Change`

| Change | Emitted SQL | Annotation |
|---|---|---|
| `AddTable` | `CREATE TABLE name (col1 type1 [NOT NULL] [DEFAULT ...], ..., PRIMARY KEY (...))` + FK constraints inline | clean |
| `DropTable` | `-- TODO: confirm DROP, then uncomment\n-- DROP TABLE name CASCADE;` | TODO ⚠ |
| `AddColumn` (has default OR nullable) | `ALTER TABLE t ADD COLUMN c TYPE [NOT NULL] [DEFAULT v];` | clean |
| `AddColumn` (NOT NULL, no default) | `-- TODO: NOT NULL without default — backfill required\n-- ALTER TABLE t ADD COLUMN c TYPE NOT NULL;` | TODO ⚠ |
| `DropColumn` | `-- TODO: confirm DROP, destructive\n-- ALTER TABLE t DROP COLUMN c;` | TODO ⚠ |
| `AlterColumnType` (widening) | `ALTER TABLE t ALTER COLUMN c TYPE new_type;` | clean |
| `AlterColumnType` (narrowing or cross-family) | `-- TODO: cast may lose data\n-- ALTER TABLE t ALTER COLUMN c TYPE new_type USING c::new_type;` | TODO ⚠ |
| `AlterColumnNullable` (true→false) | `-- TODO: backfill NULLs first\n-- ALTER TABLE t ALTER COLUMN c SET NOT NULL;` | TODO ⚠ |
| `AlterColumnNullable` (false→true) | `ALTER TABLE t ALTER COLUMN c DROP NOT NULL;` | clean |
| `AlterColumnDefault` | `ALTER TABLE t ALTER COLUMN c SET DEFAULT v;` (or `DROP DEFAULT`) | clean |
| `AddPrimaryKey` | `ALTER TABLE t ADD PRIMARY KEY (c1, c2);` | clean if columns NOT NULL |
| `DropPrimaryKey` | `-- TODO: usually a structural change, review\n-- ALTER TABLE t DROP CONSTRAINT t_pkey;` | TODO ⚠ |
| `AddUnique` | `CREATE UNIQUE INDEX t_c_key ON t (c);` | clean |
| `AddForeignKey` | `ALTER TABLE t ADD CONSTRAINT fk_name FOREIGN KEY (c) REFERENCES t2 (id) ON DELETE ...;` | clean |

### Widening table (clean type changes)

| from | to (clean) |
|---|---|
| `int2` | `int4`, `int8` |
| `int4` | `int8` |
| `float4` | `float8` |
| `varchar(N)` | `varchar(M)` where M > N, or `text` |
| `text` | (no widening, but no information loss) |
| `time` | `timetz`, `timestamp` |
| `timestamp` | `timestamptz` |
| `date` | `timestamp`, `timestamptz` |
| `numeric(p,s)` | `numeric(p',s')` where p' >= p and s' >= s |

Anything else = TODO ⚠ with `USING <col>::<new_type>` cast scaffold.

### Rename detection — explicitly NOT done

Renaming `users.email` → `users.contact_email` looks identical to "drop column `email`, add column `contact_email`" without hints. Auto-detection requires fuzzy matching that is wrong as often as right.

In v0.3, never auto-detect renames. The user manually edits the diff output:

```sql
-- generated:
-- TODO: confirm DROP, destructive
-- ALTER TABLE users DROP COLUMN email;
ALTER TABLE users ADD COLUMN contact_email text NOT NULL DEFAULT '';

-- user edits to:
ALTER TABLE users RENAME COLUMN email TO contact_email;
```

`-- TODO:` markers make this rewrite obvious.

### Output skeleton example

For a diff that adds `users.deleted_at`, removes `users.legacy_id`, and narrows `users.bio` from `text` to `varchar(500)`:

```sql
-- Migration scaffold generated by `sntl migrate diff`
-- Generated: 2026-05-09T14:00:00Z
-- Source:    .sentinel/schema.toml vs DATABASE_URL
-- Total: 3 changes (1 clean, 2 TODO)
--
-- Review TODO comments and remove the leading `-- ` to apply.

ALTER TABLE users ADD COLUMN deleted_at timestamptz NULL;

-- TODO: confirm DROP, destructive
-- ALTER TABLE users DROP COLUMN legacy_id;

-- TODO: cast may lose data (text → varchar(500))
-- ALTER TABLE users ALTER COLUMN bio TYPE varchar(500) USING bio::varchar(500);
```

### Algorithmic complexity

- `O(N + M)` where N = tables in cache, M = tables in DB.
- Inside each table: `O(C₁ + C₂)` for column diff.
- No graph traversal, no fixpoint — single linear pass.
- Pure function; easy to unit-test with hand-built `Schema` literals.

## 5. Error handling + testing

### Error enum

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO on {path}: {source}")]
    Io { path: PathBuf, #[source] source: std::io::Error },

    #[error("invalid migration folder name `{name}`: expected YYYYMMDD_HHMMSS_<snake_case>")]
    InvalidName { name: String },

    #[error("migration `{pending}` has timestamp before highest applied `{highest_applied}`")]
    OutOfOrder { pending: Version, highest_applied: Version },

    #[error("migration `{version}` failed at SQL line {line}: {source}")]
    ApplyFailed { version: Version, line: u32, #[source] source: driver::Error },

    #[error("checksum mismatch for applied migration `{version}` — file modified after apply")]
    ChecksumDrift { version: Version, file: String, recorded: String },

    #[error("could not acquire migration lock — another process is migrating")]
    LockBusy,

    #[error("driver error: {0}")]
    Driver(#[from] driver::Error),

    #[error("schema introspection failed: {0}")]
    Introspect(#[from] sntl_schema::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### User-facing error UX

Errors print via `ui::err()` (owo-colors red ✗) plus help context:

```
✗ migration `20260509_140000_drop_legacy/up.sql` failed at line 12

  ALTER TABLE users DROP COLUMN legacy_id;
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

  Server: column "legacy_id" of relation "users" does not exist (SQLSTATE 42703)

  The migration's transaction was rolled back. Fix up.sql and rerun.
  All 3 prior migrations remain applied.
```

```
✗ out-of-order migration: 20260507_080000_add_posts precedes
  highest applied 20260508_120000_add_users

  Either rebase your branch and rename the migration:

    git mv migrations/20260507_080000_add_posts \
           migrations/20260509_150000_add_posts

  ...or roll back the more recent migration manually if it was applied
  in error.
```

### Testing strategy

| Layer | Test type | What |
|---|---|---|
| `migration::Version` | unit | parse "YYYYMMDD_HHMMSS_name", compare ordering, reject bad names |
| `discover` | unit | tempdir with valid + invalid folders → returns sorted Vec, errors on malformed |
| `checksum` | unit | sha256 deterministic, sensitive to whitespace/comments |
| `tracking` | live-PG | `ensure()` is idempotent, `record()` + `applied_versions()` round-trip |
| `runner` | live-PG | apply pending applies in order, no-op on rerun, advisory lock blocks 2nd process, transaction rollback on error |
| `runner` | live-PG | `up.notx.sql` skips transaction (verify with `CREATE INDEX CONCURRENTLY`) |
| `runner` | unit | strict ordering rejected with clear error |
| `diff::compare` | unit | hand-built Schema literals → expected `Vec<Change>` for every Change variant |
| `diff::emit` | unit | each Change variant → expected SQL substring + TODO count |
| `refresh` | live-PG | post-apply `pull_schema` writes `.sentinel/schema.toml` matching new state |
| macro `migrate!()` | trybuild | bundle 2 migrations, verify `from_static` Migrator runs them |
| `sntl-cli` `migrate add` | integration | tempdir + assert folder + file template |
| `sntl-cli` `migrate run` | live-PG | end-to-end — add, run, info, verify |
| `sntl-cli` `migrate diff` | live-PG | drift DB from cache → diff produces expected SQL skeleton |

### Concurrency / safety tests (live-PG)

- **Lock blocks**: spawn task A acquires lock + sleeps 2s; task B's `run()` waits, succeeds after A releases.
- **No-op idempotency**: `run()` × 2 in succession; second is fast no-op, `_sntl_migrations` unchanged.
- **Partial failure**: 2-statement migration where statement #2 fails; rollback verified via `applied_versions()` not containing version, also `users` table state unchanged.
- **Checksum drift**: apply migration, modify `up.sql` content, re-run `info` → ⚠ flag for that version.

### CI integration

- Add `sntl-migrate` to `--workspace --all-features` test invocation.
- Live-PG tests run under existing `postgresql.yml` workflow (gated by `DATABASE_URL`).
- Coverage workflow adds `sntl-migrate/` to `--ignore-filename-regex` initially; revisit when integration tests sufficient (Coverage 2.0 follow-up).

### Coverage targets

- Unit tests: ≥95% line for `migration.rs`, `discover.rs`, `checksum.rs`, `diff/*`.
- Live-PG paths (runner, tracking, refresh): ≥80% line via integration tests.
- macro_support / `migrate!` macro: trybuild pass case is enough — proc-macro coverage limitations same as PR #14.

## 6. Open items / verification before implementation

- Confirm `sentinel-driver` exposes a public `advisory_lock(id) -> AdvisoryLockGuard` API on `Connection` (driver report mentioned `PgAdvisoryLockGuard`; verify exact method name + lock ID type).
- Confirm `sntl-schema::introspect::pull_schema` returns enough table/column metadata for diff comparison, including column defaults and FK targets.
- Pick the lock ID constant — proposal `0x736e_746c_6d67_7274_u64` ("sntlmgrt" bytes). Check it does not collide with any well-known third-party lock (e.g. pg_repack uses 16-bit constants; we use 64-bit, low collision risk).
- Decide whether `Migrator::from_dir` should fail eagerly on first invalid folder name or skip with a warning. Proposal: fail eagerly, return `Error::InvalidName`.
