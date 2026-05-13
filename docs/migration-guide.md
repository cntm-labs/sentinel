# Migration Guide — `sntl-migrate` v0.3

`sntl-migrate` is Sentinel's forward-only SQL migration tool. It ships as a
library (`sntl_migrate`) for runtime use plus a `sntl migrate ...`
subcommand for the day-to-day workflow.

This guide assumes a workspace that has already run `sntl init`.

## Quick start

```sh
sntl init                  # if you haven't already
sntl migrate add "create users"
# edit migrations/20260513_142500_create_users/up.sql
sntl migrate run
```

`migrate add` scaffolds `migrations/<UTC timestamp>_<sanitised_name>/up.sql`
with a header comment. `migrate run` applies every pending migration in
version order, then refreshes `.sentinel/schema.toml` so the compile-time
`sntl::query!()` cache stays in sync.

## Daily workflow

1. **Add** — `sntl migrate add "<short description>"`.
   The CLI lowercases the name, replaces non-alphanumeric chars with
   underscores, and prefixes it with the current UTC `YYYYMMDD_HHMMSS`.
2. **Edit** — open `migrations/<name>/up.sql` and write the SQL. The file
   runs inside a single `BEGIN/COMMIT` by default.
3. **Apply** — `sntl migrate run`. The runner takes a PostgreSQL advisory
   lock keyed on the ASCII bytes `"sntlmgrt"` (`0x736e_746c_6d67_7274`)
   so concurrent deployments serialise instead of double-applying.
4. **Verify** — `sntl migrate info --all` to see applied (✓), pending
   (◯), and drifted (⚠) migrations.

## Production embedding

For a deploy where the migrations folder isn't shipped alongside the
binary, embed the SQL at compile time:

```rust
use sntl_migrate::migrate;

let migrator = migrate!("./migrations");
let report = migrator.run(&pool).await?;
```

The `migrate!()` proc-macro walks the folder during compilation, calls
`include_str!` on each `up.sql` / `up.notx.sql`, and emits a fully
populated `Migrator`. Production binaries never read the filesystem for
migrations.

## Non-transactional DDL

Some PostgreSQL statements cannot run inside a transaction:
`CREATE INDEX CONCURRENTLY`, `REFRESH MATERIALIZED VIEW CONCURRENTLY`,
`VACUUM`, `CREATE DATABASE`, etc. Name the file `up.notx.sql` and the
runner will execute it directly without wrapping it in `BEGIN/COMMIT`.

```
migrations/
├── 20260513_142500_create_users/
│   └── up.sql
└── 20260513_153000_index_email/
    └── up.notx.sql
```

## Strict ordering

`sntl migrate run` refuses to apply a migration whose timestamp is older
than the highest-applied one. If your branch's migration timestamp gets
overtaken on `main`, rebase your branch and rename the folder with a
fresh timestamp before applying.

## Concurrent deploys

Two `sntl migrate run` processes hitting the same database serialise
through the advisory lock. The second process waits, then sees every
migration already applied and exits as a no-op. Roll-out automation can
run `sntl migrate run` on every replica without coordination logic.

## Diff workflow

Sentinel keeps the source-of-truth schema in `.sentinel/schema.toml`
(refreshed by `sntl migrate run` and `sntl prepare`). If the live DB
drifts (for example, someone applied a hot-fix outside the migration
pipeline) `sntl migrate diff` will compare cache vs DB and emit a SQL
scaffold:

```sh
sntl migrate diff --out hotfix_recovery
```

The generated `migrations/<ts>_hotfix_recovery/up.sql` contains:

- Clean SQL for safe operations: `CREATE TABLE`, widening type casts
  (e.g. `int4 → int8`), adding nullable columns, adding default columns,
  `DROP NOT NULL`, `ADD/DROP DEFAULT`, `ADD PRIMARY KEY`, `ADD UNIQUE`.
- Commented-out SQL with `-- TODO:` markers for destructive or lossy
  operations: `DROP TABLE`, `DROP COLUMN`, narrowing casts,
  `SET NOT NULL` without backfill, dropping primary keys, dropping
  uniques.

Review every TODO, uncomment what's safe to apply, then run
`sntl migrate run`.

**Out of v0.3 scope:** foreign-key, enum, and composite-type diffs.
`pull_schema` does not yet populate these, so the diff will not flag
drift in those areas.

## CI integration

`sntl migrate verify` exits non-zero if any applied migration's on-disk
checksum has changed since it was first applied — a fast smoke test to
catch accidental edits of historical migrations. Wire it into the deploy
pipeline ahead of `migrate run`:

```yaml
- name: Verify migrations
  run: sntl migrate verify
- name: Apply migrations
  run: sntl migrate run
```

## Limitations (v0.3)

- **No `down.sql` / revert.** Sentinel takes the forward-only stance: a
  rollback is just another forward migration that re-creates state.
- **No FK / enum / composite diff.** `pull_schema` doesn't populate
  these yet; the diff scaffolder will skip them silently.
- **No rename detection.** A column rename shows up as a `DropColumn` +
  `AddColumn`, with both flagged TODO for review.

## Reference

- `Migrator::from_dir(path)` — discover at runtime.
- `sntl_migrate::migrate!("./migrations")` — embed at compile time.
- `Migrator::with_refresh(conn_str, cache_dir)` — auto-refresh
  `<cache_dir>/schema.toml` after apply.
- `Migrator::run(&pool)` — apply pending; returns `MigrationReport`.
- `Migrator::info(&pool)` — applied / pending / drifted status.
- `sntl_migrate::SNTL_MIGRATE_LOCK_ID` — the advisory-lock key.
