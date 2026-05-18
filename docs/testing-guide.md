# Testing Guide — `#[sntl::test]`

`#[sntl::test]` is Sentinel's fixture-isolated test harness. Each invocation
gets a fresh PostgreSQL database cloned from a template at PG-level speed
(~10 ms per test). Migrations run once per process and are reused across
tests via a template DB. Fixtures are applied per test, in order, inside
a single transaction.

This guide covers setup, all three forms of the attribute, environment
variables, required permissions, and debugging leaked databases.

---

## 30-second quickstart

Add `sntl` to `[dev-dependencies]` if it isn't already:

```toml
[dev-dependencies]
sntl = "0.5"
anyhow = "1"
```

Write a test:

```rust
#[sntl::test]
async fn empty_db(pool: sentinel_driver::Pool) -> anyhow::Result<()> {
    let n: i64 = sntl::query_scalar!("SELECT 1::int8").fetch_one(&pool).await?;
    assert_eq!(n, 1);
    Ok(())
}
```

Run it:

```sh
SNTL_TEST_DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/postgres cargo test
```

The test gets a fresh empty database, runs the body, and drops the database on
success. No setup boilerplate, no `TRUNCATE`, no shared state.

---

## The three forms

### No arguments — empty database

```rust
#[sntl::test]
async fn empty_db(pool: sentinel_driver::Pool) -> anyhow::Result<()> {
    // Fresh DB with no schema. Useful for tests that construct their own
    // tables or just need a live connection.
    let n: i64 = sntl::query_scalar!("SELECT COUNT(*) FROM pg_tables")
        .fetch_one(&pool).await?;
    assert!(n >= 0);
    Ok(())
}
```

### `migrations` — schema applied

```rust
#[sntl::test(migrations = "./tests/migrations")]
async fn with_schema(pool: sentinel_driver::Pool) -> anyhow::Result<()> {
    // DB has all migrations in ./tests/migrations applied.
    // Migrations run once per process; all tests sharing the same path
    // clone from the cached template.
    sntl::query!("INSERT INTO users (id, name) VALUES (1, 'alice')")
        .execute(&pool).await?;
    Ok(())
}
```

The path is relative to `CARGO_MANIFEST_DIR` (the crate root, same as what
`include_str!` uses). Forward slashes work on all platforms.

### `migrations` + `fixtures` — seeded database

```rust
#[sntl::test(migrations = "./tests/migrations", fixtures("users", "posts"))]
async fn full_setup(pool: sentinel_driver::Pool) -> anyhow::Result<()> {
    // DB has migrations applied, then tests/fixtures/users.sql and
    // tests/fixtures/posts.sql executed in that order, inside one transaction.
    let posts = sntl::query_as!(Post, "SELECT * FROM posts WHERE user_id = 1")
        .fetch_all(&pool).await?;
    assert_eq!(posts.len(), 3);
    Ok(())
}
```

Fixture names map to `<CARGO_MANIFEST_DIR>/tests/fixtures/<name>.sql`. The
order declared in `fixtures(...)` is the order they run — relevant when
one fixture depends on rows inserted by another.

---

## Directory layout

```
your-crate/
├── Cargo.toml
├── src/
│   └── ...
└── tests/
    ├── fixtures/
    │   ├── users.sql       # INSERT rows for the users table
    │   └── posts.sql       # INSERT rows for the posts table (may depend on users)
    ├── migrations/
    │   └── 20260514_120000_init/
    │       └── up.sql      # CREATE TABLE users (...); CREATE TABLE posts (...);
    └── my_test.rs          # #[sntl::test] functions live here
```

Fixtures are plain SQL files. Each one is executed verbatim; there is no
special syntax. Because all fixtures for a single test run in one transaction,
a failure in any fixture rolls back the entire seed, and the test body never
runs.

---

## Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `SNTL_TEST_DATABASE_URL` | Falls back to `DATABASE_URL` | Admin connection URL. Must point at a database the test role can `CREATE DATABASE` from (`postgres` system DB is typical). |
| `SNTL_TEST_KEEP_DBS` | unset | Set to `1` to disable auto-drop of test databases. Useful when you need to inspect a passing test's DB state. |

If neither `SNTL_TEST_DATABASE_URL` nor `DATABASE_URL` is set, tests that use
`#[sntl::test]` skip silently — a message is printed to stderr and the test
is reported as passed (same contract as the existing `require_pg!` macro).

**Do not** point `SNTL_TEST_DATABASE_URL` at your application database. The
harness creates and drops databases under the connected user. The `postgres`
system database or a dedicated `sentinel_test` database are the right targets.

---

## Required PostgreSQL permissions

The role used in `SNTL_TEST_DATABASE_URL` must have `CREATEDB`. This is the
minimum required to create template databases and per-test clones.

```sql
CREATE ROLE sentinel WITH LOGIN PASSWORD 'sentinel_test' CREATEDB;
```

If the role lacks `CREATEDB`, the first test invocation will fail with a clear
error pointing at the missing permission and the env var to check.

Superuser is not required. `CREATEDB` is sufficient for everything the harness
does.

---

## Local PostgreSQL setup

### Podman (preferred)

```bash
podman run -d --name sntl-test-pg \
  -e POSTGRES_USER=sentinel \
  -e POSTGRES_PASSWORD=sentinel_test \
  -e POSTGRES_DB=postgres \
  -p 5432:5432 \
  postgres:17
```

Then export the URL and run tests:

```sh
export SNTL_TEST_DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/postgres
cargo test
```

Stop and remove the container when done:

```sh
podman stop sntl-test-pg && podman rm sntl-test-pg
```

### Docker Compose

The repository ships a `docker-compose.yml` for PG 16. Use it if you prefer
Compose or if the project's CI already targets PG 16:

```sh
docker compose up -d
export SNTL_TEST_DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/postgres
cargo test
```

---

## Template caching — why the first run is slow

The first `#[sntl::test]` invocation in a process that references a migrations
directory does the following:

1. Computes a SHA-256 of the canonical path to the migrations directory and
   takes the first 8 hex characters as a key: `_sntl_tmpl_<key>`.
2. Drops any existing `_sntl_tmpl_<key>` database (stale template from a
   previous run).
3. Creates a fresh `_sntl_tmpl_<key>` database.
4. Connects to it and applies every migration end-to-end.
5. Marks the database as a PG-level template
   (`ALTER DATABASE ... WITH ALLOW_CONNECTIONS FALSE`).

This template-build step takes as long as your migrations take — usually a
few seconds for typical schemas.

Every subsequent `#[sntl::test]` in the same `cargo test` process that
references the same migrations directory clones from `_sntl_tmpl_<key>` via
`CREATE DATABASE <test_db> TEMPLATE _sntl_tmpl_<key>`. PG copies the data
files at the filesystem level, which typically takes ~10 ms regardless of
schema size.

The template is **per-process**: each `cargo test` invocation rebuilds it. If
you want to skip the rebuild across runs (for a long-running dev session), set
`SNTL_TEST_KEEP_DBS=1` — the template will not be dropped on startup. This is
an advanced workflow and not the default; stale templates can cause confusing
failures if migrations change between runs.

---

## Debugging — finding and inspecting leaked databases

When a test body returns `Err(...)` or panics, the per-test database is **not**
dropped. The harness prints the connection URL to stderr so you can inspect the
state:

```
test with_schema ... FAILED
  leaked test DB: psql postgres://sentinel:sentinel_test@localhost:5432/_sntl_with_schema_a3f2
```

Connect with `psql` (or any PG client) and inspect rows, check constraints,
or replay queries against the exact data the test saw.

When you want to inspect a database from a test that *passed* (so it was
auto-dropped), set `SNTL_TEST_KEEP_DBS=1` before re-running:

```sh
SNTL_TEST_KEEP_DBS=1 SNTL_TEST_DATABASE_URL=... cargo test my_test_name
```

### Listing leaked databases

```sh
psql $SNTL_TEST_DATABASE_URL -c "\l" | grep _sntl_
```

### Cleaning up test databases

Drop a per-test DB normally:

```sql
DROP DATABASE _sntl_with_schema_a3f2;
```

Template databases (`_sntl_tmpl_*`) are marked `datistemplate = true` by PG,
which prevents a plain `DROP DATABASE`. Clear the flag first:

```sql
UPDATE pg_database
   SET datistemplate = false
 WHERE datname = '_sntl_tmpl_<key>';

DROP DATABASE _sntl_tmpl_<key>;
```

Replace `<key>` with the 8-character hash shown in the database name.

---

## Limitations (v0.5)

| Limitation | Detail |
|---|---|
| **PostgreSQL only** | `#[sntl::test]` targets PostgreSQL exclusively. Multi-database support is not planned for v0.5. |
| **CREATEDB required** | The test role must have `CREATEDB`. Shared development databases where you can't grant `CREATEDB` are not suitable. Use a dedicated test PG instance. |
| **First test per process is slow** | Template build cost equals migration runtime. All later tests in the same `cargo test` invocation share the template and pay only the ~10 ms clone cost. |
| **No transaction-level rollback isolation** | Each test gets a separate database (PG-level isolation). There is no `ROLLBACK`-based teardown between tests. This is by design — it avoids savepoint complexity and DDL-in-tx constraints. |
| **No `down.sql` support** | `sntl-migrate` is forward-only. The test harness applies migrations in order and has no concept of reverting them. |
| **Test names must be unique within a crate** | Database names are derived from test function names. Duplicate names across modules would collide; the macro errors at compile time if a collision is detected. |
