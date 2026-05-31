# `#[sntl::reducer]` Guide

A transactional reducer wraps an async fn in `BEGIN` / `COMMIT` / `ROLLBACK` with
automatic rollback on `Err`, automatic rollback on panic, and observability
events. It is the recommended way to write multi-query atomic operations in
Sentinel v0.6+.

## 30-second quickstart

```rust
use sntl::driver::Connection;

#[sntl::reducer]
async fn transfer(conn: &mut Connection, from: i32, to: i32, amount: i64)
    -> sntl::Result<()>
{
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

#[tokio::main]
async fn main() -> sntl::Result<()> {
    let pool = /* ... build a Pool ... */;
    let mut conn = pool.acquire().await?;
    transfer(&mut conn, 1, 2, 100).await?;
    Ok(())
}
```

## Three forms

### 1. Default — driver default isolation (ReadCommitted)

```rust
#[sntl::reducer]
async fn name(conn: &mut Connection, ...) -> Result<R, E> { ... }
```

### 2. Explicit isolation

```rust
#[sntl::reducer(isolation = "serializable")]
async fn name(conn: &mut Connection, ...) -> Result<R, E> { ... }
```

Valid values: `"read_committed"`, `"repeatable_read"`, `"serializable"`.

### 3. Panic safety (automatic — no extra syntax)

If the body panics, the transaction is rolled back, `ReducerRollback` fires
with `error: "panic"`, and the panic is re-raised. The connection is left in
a clean state.

## Inside the body — use raw driver methods

The body holds `conn: &mut Connection`. The `sntl::query!()` / `query_as!()` /
`query_unchecked!()` macro family is **incompatible** with `&mut Connection` in
v0.6 because the driver's `GenericClient` trait is implemented for owned
`Connection` (and `PooledConnection`) only — not for `&mut Connection`. Use the
driver's raw `conn.execute(...)`, `conn.query(...)`, `conn.query_one(...)`,
`conn.query_opt(...)` methods inside the reducer body. They take a `&str` and
`&[&(dyn ToSql + Sync)]` params slice.

Tracked for v0.7: a blanket `impl GenericClient for &mut T` on the driver side
will let `query!()` work transparently inside reducers.

## Observability

Every `#[sntl::reducer]` fn emits three events through the configured
`Instrumentation` impl:

| Event              | When                                                          |
|--------------------|---------------------------------------------------------------|
| `ReducerBegin`     | Right after `BEGIN`, before the body runs                     |
| `ReducerCommit`    | After successful `COMMIT` (`duration` field = elapsed time)   |
| `ReducerRollback`  | After `ROLLBACK` (error returned or panic caught)             |

Use `sntl::observability::install_default_tracing(pool)` to forward these to
any `tracing`-compatible backend. See [observability-guide.md](observability-guide.md).

## Limitations (v0.6)

- **First arg must be `&mut Connection`.** Pool callers must
  `let mut conn = pool.acquire().await?;` and pass `&mut conn`. The
  `&Pool` first-arg form is tracked for v0.7+.
- **Macro layer not callable inside the body.** Use raw driver methods.
  Documented above.
- **Nested reducers are NOT supported.** Calling one `#[sntl::reducer]` fn
  from inside another opens a second `BEGIN` on a connection already in
  a transaction; PostgreSQL warns and the inner `COMMIT` becomes a no-op.
  Use savepoints (`SAVEPOINT` SQL) if you need partial rollback.
- **`#![forbid(unsafe_code)]` is incompatible.** The macro emits an unsafe
  pointer reborrow to support `catch_unwind`. If your crate forbids
  unsafe, use the explicit `Transaction::begin` form instead:

  ```rust
  let mut tx = sntl::Transaction::begin(&mut conn).await?;
  /* queries through tx.conn() */
  tx.commit().await?;
  ```

- **Rollback failures poison the connection.** If `conn.rollback()` itself
  fails after a `Err`/panic (e.g. the network dropped), the connection is
  left mid-transaction. The next user from the pool will see "current
  transaction is aborted" until a successful `ROLLBACK`. Pool poisoning on
  rollback failure is tracked for v0.7.
- **`E: Display` required.** The macro emits `format!("{e}")` to build the
  `error` field of `ReducerRollback`. `sntl::Error`, `anyhow::Error`, and
  most `thiserror`-derived enums work out of the box.
