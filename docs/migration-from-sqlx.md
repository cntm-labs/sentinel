# Migrating from `sqlx::query!` to `sntl::query!`

The `sntl::query!()` family is API-compatible with sqlx's macros for the
common cases. This guide walks through the five steps a typical sqlx
project takes to switch.

> All examples assume PostgreSQL. Sentinel does not target other engines in v0.x.

---

## 1. Swap the dependency

```diff
 # Cargo.toml
 [dependencies]
-sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "chrono"] }
+sntl = "0.1"
 tokio = { version = "1", features = ["full"] }
```

Sentinel's macros pull `serde_json`, `chrono`, `uuid`, and `rust_decimal`
through their generated code. Make sure those crates are reachable from
your `Cargo.toml` if you use the corresponding PostgreSQL types.

## 2. Move from `DATABASE_URL` to `sentinel.toml`

sqlx looks at `DATABASE_URL` at compile time when the offline cache is
absent. Sentinel always reads from `.sentinel/` and only consults
`sentinel.toml` to decide which database to introspect when you run
`sntl prepare`.

```toml
# sentinel.toml at the workspace root
[database]
url = "postgres://app:app@localhost:5432/app_dev"

[macros]
strict_nullable = true   # treat unknown nullability as Option<T> (default)
```

`SENTINEL_DATABASE_URL`, `SENTINEL_OFFLINE`, and `SENTINEL_CACHE_DIR`
override the file at runtime.

## 3. Generate the cache

```sh
cargo install --path sntl-cli   # or `cargo install sntl-cli`
sntl prepare                    # writes .sentinel/schema.toml + queries/*.json
git add .sentinel
```

CI builds use the committed cache without ever talking to the database.
`sntl check` (also CI-friendly) fails if any source-side query is missing
from the cache.

## 4. Translate macro call sites

For the bread-and-butter cases the syntax is unchanged:

```diff
-let user = sqlx::query!("SELECT id, email FROM users WHERE id = $1", id)
-    .fetch_one(&pool)
-    .await?;
+let user = sntl::query!("SELECT id, email FROM users WHERE id = $1", id)
+    .fetch_one(&pool)
+    .await?;
```

`fetch_*` / `execute` accept any `impl GenericClient + Send` — pass `&pool`,
`&mut conn`, or `&mut PooledConnection` interchangeably (v0.5).

Type-targeted form:

```diff
-let users: Vec<User> = sqlx::query_as!(User, "SELECT id, email FROM users").fetch_all(&pool).await?;
+let users: Vec<User> = sntl::query_as!(User, "SELECT id, email FROM users").fetch_all(&pool).await?;
```

`sqlx::query_scalar!` → `sntl::query_scalar!` (single-column projection).

`sqlx::query_file!` and `sqlx::query_file_as!` work the same way; the path
is interpreted relative to `CARGO_MANIFEST_DIR`.

If you have one-off SQL that you intentionally do not want to cache, use
the escape hatches:

```rust
let _ = sntl::query_unchecked!("SET search_path = public").execute(&pool).await?;
```

### Streaming large result sets

`sqlx::query!(...).fetch(&mut conn)` returns a `Stream`. Sentinel's
equivalent lives on `QueryExecution`:

```diff
-let mut stream = sqlx::query!("SELECT id FROM big_table").fetch(&mut conn);
-while let Some(row) = stream.try_next().await? {
+let mut stream = sntl::query!("SELECT id FROM big_table").fetch_stream(&mut conn).await?;
+while let Some(row) = stream.next().await? {
     let id: i32 = row.try_get(0)?;
 }
```

`fetch_stream` requires `&mut Connection` specifically — the returned
`RowStream` borrows the connection for its lifetime, so pool-acquired
handles (`&pool`) are not yet supported here.

Use `sntl::query_unchecked!("...").into_stream().fetch_stream(&mut conn)` if
you want the streaming path on a query you have not cached.

## 5. Lift sqlx's nullability annotations

sqlx encodes nullability hints inside the SQL string (`SELECT id AS "id!"`,
etc). Sentinel takes them as macro arguments instead, which is more
discoverable and works on any column without rewriting the query:

```diff
-let row = sqlx::query!("SELECT email AS \"email!\" FROM users WHERE id = $1", id)
+let row = sntl::query!(
+    "SELECT email FROM users WHERE id = $1",
+    id,
+    non_null = [email]
+)
     .fetch_one(&mut conn)
     .await?;
```

`nullable = [...]` is the inverse — force a column the inferencer thought
was non-null to `Option<T>`.

---

## 6. Swap `#[sqlx::test]` for `#[sntl::test]`

Sentinel ships a fixture-isolated test harness that creates a fresh
PostgreSQL database per test via `CREATE DATABASE ... TEMPLATE` — the same
shape as `sqlx::test`.

```diff
-#[sqlx::test(migrations = "./migrations", fixtures("users", "posts"))]
-async fn user_has_two_posts(pool: sqlx::PgPool) -> sqlx::Result<()> {
+#[sntl::test(migrations = "./migrations", fixtures("users", "posts"))]
+async fn user_has_two_posts(pool: sentinel_driver::Pool) -> anyhow::Result<()> {
     // … assertions …
     Ok(())
 }
```

Differences:

- **Admin URL via `SNTL_TEST_DATABASE_URL`** (fallback `DATABASE_URL`).
  Point it at the `postgres` system DB so the test role can CREATE DATABASE.
- **Migrations are sntl-migrate folders**, not sqlx's flat `up.sql` files.
  See [migration-guide.md](migration-guide.md).
- **Body must return `anyhow::Result<()>`.** Sentinel does not use a
  bespoke `Result` alias.
- **First test per process is slow** while it builds the template DB;
  subsequent clones are ~milliseconds. Detail in
  [testing-guide.md](testing-guide.md).

## What's different on purpose

- **No `DATABASE_URL` at compile time.** The cache is the source of truth.
  `sntl prepare` is the only step that touches the database.
- **Pipelined batches are first-class.** `sntl::query_pipeline!()` packs
  N queries into one network round-trip; sqlx has no equivalent macro.
- **Cache hash is part of the diagnostic.** Cache misses tell you the
  exact `.sentinel/queries/<HASH>.json` path so the fix is one `sntl
  prepare` away.
- **OIDs come from `sntl prepare`, not the call site.** The macro emits
  `query_typed_*` calls with the cached parameter OIDs, skipping a Parse
  round-trip versus untyped `query` paths.
- **`Pool` is a first-class macro argument (v0.5).** No need to acquire a
  connection by hand for one-off queries — `&pool` works everywhere
  `&mut conn` does, except `fetch_stream` (which borrows the connection).
- **Streaming, transactions, and `#[sntl::test]`** are all built on the
  same `Instrumentation` trait — see
  [observability-guide.md](observability-guide.md) for the wire-level
  events you can hook into.

## 7. Replacing `sqlx::Transaction` with `#[sntl::reducer]`

sqlx:

```rust
let mut tx = pool.begin().await?;
sqlx::query!("UPDATE accounts SET balance = balance - $1 WHERE id = $2", amount, from)
    .execute(&mut *tx).await?;
sqlx::query!("UPDATE accounts SET balance = balance + $1 WHERE id = $2", amount, to)
    .execute(&mut *tx).await?;
tx.commit().await?;
```

Sentinel:

```rust
use sntl::driver::Connection;

#[sntl::reducer]
async fn transfer(conn: &mut Connection, from: i32, to: i32, amount: i64) -> sntl::Result<()> {
    conn.execute(
        "UPDATE accounts SET balance = balance - $1 WHERE id = $2",
        &[&amount, &from],
    ).await?;
    conn.execute(
        "UPDATE accounts SET balance = balance + $1 WHERE id = $2",
        &[&amount, &to],
    ).await?;
    Ok(())
}

let mut conn = pool.acquire().await?;
transfer(&mut conn, 1, 2, 100).await?;
```

The reducer macro emits the `BEGIN`/`COMMIT`/`ROLLBACK` boilerplate for you,
adds panic safety (automatic rollback if the body panics), and emits
observability events. Inside the body, use the driver's raw `conn.execute(...)`
/ `conn.query(...)` methods — the `sntl::query!()` macro family does not yet
work with `&mut Connection` (tracked for v0.7). See
[`docs/reducer-guide.md`](reducer-guide.md).
