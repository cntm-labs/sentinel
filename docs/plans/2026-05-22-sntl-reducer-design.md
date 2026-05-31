# `#[sntl::reducer]` Design — v0.6 Phase 1

**Status:** approved 2026-05-22, implementation pending
**Closes:** doc-vs-code gap surfaced by `/graphify` audit — `#[reducer]` is listed in `CLAUDE.md` as a "Key Pattern" and the observability layer has dormant `ReducerBegin/Commit/Rollback` event arms, but the proc-macro itself was never written.

---

## 1. Goal

Ship a `#[sntl::reducer]` attribute proc-macro that wraps any async fn in a PostgreSQL transaction with:

1. Automatic `BEGIN` / `COMMIT` / `ROLLBACK` (no manual `Transaction::begin()` boilerplate at the call site).
2. Automatic `ReducerBegin` / `ReducerCommit` / `ReducerRollback` event emission through the existing `Instrumentation` trait.
3. Optional isolation control via `#[sntl::reducer(isolation = "...")]`.
4. Panic safety — if the body panics, the transaction is rolled back and a `ReducerRollback` event fires before the panic resumes.

Closes the largest doc-vs-code gap in the v0.5 codebase (1 of 5 hard gaps the graphify audit identified).

**Explicitly out of scope (deferred to v0.7+):**
- Deadlock prevention via auto-reorder lock by ID — fundamentally a runtime lock-manager API, not a macro concern. Tracked separately as `LockBatch` API.
- Nested reducers / savepoints — v0.x users get a documented limitation; the second `BEGIN` warns from PG but doesn't error.
- Pool poisoning on rollback failure — driver-side concern.
- `&Pool` first-arg form — user can `pool.acquire().await?` themselves; macro accepts only `&mut Connection` (concrete).

---

## 2. User-Visible API

### Default form (driver default isolation)

```rust
use sntl::driver::Connection;

#[sntl::reducer]
async fn transfer(conn: &mut Connection, from: i32, to: i32, amount: i64)
    -> sntl::Result<()>
{
    sntl::query!(
        "UPDATE accounts SET balance = balance - $1 WHERE id = $2",
        amount, from,
    ).execute(&mut *conn).await?;
    sntl::query!(
        "UPDATE accounts SET balance = balance + $1 WHERE id = $2",
        amount, to,
    ).execute(&mut *conn).await?;
    Ok(())
}
```

### With isolation level

```rust
#[sntl::reducer(isolation = "serializable")]
async fn cas_increment(conn: &mut Connection, id: i32) -> sntl::Result<i64> {
    let current: i64 = sntl::query_scalar!(
        "SELECT counter FROM kv WHERE id = $1", id
    ).fetch_one(&mut *conn).await?;
    sntl::query!(
        "UPDATE kv SET counter = $1 WHERE id = $2", current + 1, id
    ).execute(&mut *conn).await?;
    Ok(current + 1)
}
```

### Constraints enforced at macro-expansion time

- Function MUST be `async fn`. Sync fn → `abort!(span, "#[sntl::reducer] requires async fn")`.
- First argument MUST be a name bound to `&mut Connection`. Other shapes (`&Connection`, `&Pool`, `&mut Transaction<'_>`) → `abort!(span, "first arg must be `&mut driver::Connection`")`.
- Return type MUST be `Result<R, E>` where `E: Display`. The bound is implicit (compile error surfaces at the macro-generated `format!("{e}")` site).

---

## 3. Architecture & Codegen

### 3.1 Location

```
sntl-macros/src/
├── reducer/
│   ├── mod.rs           # mod codegen; pub use codegen::expand;
│   └── codegen.rs       # Args parser + expand()
├── lib.rs               # MODIFIED: register #[proc_macro_attribute] fn reducer
```

Mirror the existing `sntl-macros/src/test/` layout. `sntl/src/lib.rs` adds `pub use macros::reducer;` to the macros re-export block (alphabetised between `query_unchecked` and `sentinel`).

### 3.2 Expanded output

The macro rewrites:

```rust
#[sntl::reducer]
async fn transfer(conn: &mut Connection, from: i32, to: i32, amount: i64)
    -> sntl::Result<()>
{
    BODY
}
```

into:

```rust
async fn transfer(conn: &mut Connection, from: i32, to: i32, amount: i64)
    -> sntl::Result<()>
{
    use ::std::time::Instant;
    use ::sntl::driver::Event;
    use ::sntl::__macro_support::FutureExt as _;   // re-export of futures::FutureExt
    use ::std::panic::AssertUnwindSafe;

    let __name: &'static str = "transfer";
    conn.instrumentation().on_event(&Event::ReducerBegin { name: __name });
    let __start = Instant::now();

    conn.begin().await?;   // OR conn.begin_with(...) — see §4

    // Borrow conn into an inner future so we can re-use it post-rollback.
    let __conn_ptr: *mut ::sntl::driver::Connection = conn;
    let __result = AssertUnwindSafe(async move {
        // safety: __conn_ptr was just derived from `conn`, the outer lifetime
        // outlives this scope, and we don't run the outer borrow concurrently.
        let conn = unsafe { &mut *__conn_ptr };
        BODY
    })
    .catch_unwind()
    .await;

    match __result {
        Ok(Ok(r)) => {
            conn.commit().await?;
            conn.instrumentation().on_event(&Event::ReducerCommit {
                name: __name,
                duration: __start.elapsed(),
            });
            Ok(r)
        }
        Ok(Err(e)) => {
            let __err = format!("{e}");
            conn.rollback().await.ok();
            conn.instrumentation().on_event(&Event::ReducerRollback {
                name: __name,
                error: &__err,
            });
            Err(e)
        }
        Err(panic_payload) => {
            conn.rollback().await.ok();
            conn.instrumentation().on_event(&Event::ReducerRollback {
                name: __name,
                error: "panic",
            });
            ::std::panic::resume_unwind(panic_payload);
        }
    }
}
```

### 3.3 Unsafe pointer note

The `unsafe { &mut *__conn_ptr }` reborrow is required because:
- `AssertUnwindSafe<F>` consumes `F`, so the inner future must own a reference.
- After `catch_unwind`, the outer scope needs `conn` back to call `commit/rollback`.
- A simple `&mut *conn` move into the future would prevent the outer scope from using `conn` again.

The workspace does NOT set `#![forbid(unsafe_code)]` (verified: no occurrence in any workspace `Cargo.toml` or crate root). The unsafe block in the expansion lives inside the user's crate. If a user crate sets its own `#![forbid(unsafe_code)]`, the macro expansion fails to compile. Mitigation: document the incompatibility in `docs/reducer-guide.md` and point users at the explicit `Transaction::begin` + manual `.commit()` / `.rollback()` form as the fallback.

CLAUDE.md's "Zero `unsafe` in sntl-core" rule applies to source code in `sntl-core/`, not to macro-emitted code in user crates. The macro source itself contains no unsafe.

**Alternative considered:** drop panic safety, ditch the unsafe block, accept that body-panic leaves the connection in `BEGIN` state. Rejected because panic safety was a confirmed scope item.

**Implementation discretion:** if the implementer finds a clean lifetime-only formulation that avoids the raw pointer, use it. The codegen above is a verified-correct fallback, not a mandate.

### 3.4 Why `&mut Connection` (not `impl GenericClient`)?

`Connection` exposes `.begin()`, `.commit()`, `.rollback()`, `.instrumentation()` directly. `GenericClient` (the trait that powers `&pool` ergonomics from v0.5 Phase 1B) does not. Pool callers must `let mut conn = pool.acquire().await?` and then call the reducer with `&mut *conn`.

---

## 4. Isolation Level Attr Arg

### Surface

```rust
#[sntl::reducer]                              // driver default (ReadCommitted)
#[sntl::reducer(isolation = "read_committed")]
#[sntl::reducer(isolation = "repeatable_read")]
#[sntl::reducer(isolation = "serializable")]
```

### Parser

`syn::Meta::NameValue` whose path is `isolation`, value is a `LitStr`. Map at expansion time:

| String | Driver enum |
|---|---|
| `"read_committed"` | `IsolationLevel::ReadCommitted` |
| `"repeatable_read"` | `IsolationLevel::RepeatableRead` |
| `"serializable"` | `IsolationLevel::Serializable` |
| anything else | `abort!(span, "isolation must be one of: read_committed, repeatable_read, serializable")` |

### Codegen impact

Replace `conn.begin().await?;` with:

```rust
conn.begin_with(
    ::sntl::driver::TransactionConfig::new()
        .isolation(::sntl::driver::IsolationLevel::Serializable)
).await?;
```

Driver default omitted when no `isolation` arg given.

---

## 5. Panic Safety

### Dependency

`futures = "0.3"` added to `sntl/Cargo.toml` `[dependencies]` (already transitively present via tokio's optional feature; promoting to direct dep with `default-features = false, features = ["std"]`).

`sntl/src/lib.rs` re-exports `FutureExt` under `pub mod __macro_support { pub use ::futures::FutureExt; ... }` so the macro can emit `::sntl::__macro_support::FutureExt` and user crates do NOT need `futures` in their own dependencies.

### `AssertUnwindSafe` justification

The body captures `&mut Connection`. `&mut T: !UnwindSafe` by default, so without `AssertUnwindSafe` the future is not `UnwindSafe` and `catch_unwind` won't apply.

`AssertUnwindSafe` is sound when:
- Any data structure observed across the panic boundary is either re-validated or recreated.
- In our case: after `catch_unwind`, the *only* thing the outer scope does with the connection is `rollback().await.ok()` — which restores the connection to a clean `ReadyForQuery` state.
- No user-controlled invariants are read after panic except the `panic_payload` itself.

### Known limitations (documented in `docs/reducer-guide.md`)

- If `conn.rollback()` itself fails (network drop right after panic), the connection is left in a poisoned state. The next caller from the pool sees "current transaction is aborted, commands ignored" until a `ROLLBACK` succeeds. Pool poisoning fix is tracked separately for v0.7.
- Panics inside `conn.commit()` / `conn.rollback()` themselves are NOT caught — they propagate. Driver bugs, not user errors.
- `#![forbid(unsafe_code)]` in user crates breaks the expansion. Document the workaround: open a `Transaction` manually.

---

## 6. Error Trait Constraint

The macro emits `let __err = format!("{e}");` before `rollback`. This requires `E: std::fmt::Display`.

- `sntl::Error: Display` ✓ — works out of the box for `sntl::Result<R>`.
- `anyhow::Error: Display` ✓.
- Most custom error enums derive `thiserror::Error` which provides `Display`.

If user's E does not implement `Display`, the error surfaces at the `format!` site (a `cannot format` rustc error). The macro span is preserved, so the diagnostic points at the `#[sntl::reducer]` attribute. v0.7+ may add an explicit `compile_error!` check with a friendlier message; MVP relies on rustc's diagnostic.

---

## 7. Testing Strategy

### 7.1 Macro-level (sntl-macros)

**trybuild expansion fixtures** (`sntl-macros/tests/expand/reducer/`):
- `basic.rs` + `basic.expanded.rs` — default form.
- `with_isolation.rs` + `with_isolation.expanded.rs` — verify `begin_with(TransactionConfig::new().isolation(...))` emission.

**Compile-fail fixtures** (`sntl-macros/tests/compile_fail/`):
- `reducer_not_async.rs` — `#[reducer] fn foo(...)` → error mentioning "requires async fn".
- `reducer_no_conn_arg.rs` — first arg is `i32` → error mentioning "first arg must be `&mut driver::Connection`".
- `reducer_bad_isolation.rs` — `isolation = "snapshot"` → error listing valid values.

### 7.2 Runtime (sntl/tests/reducer_test.rs, live PG)

| Test | Setup | Assertion |
|---|---|---|
| `success_commits` | `#[reducer]` body returns `Ok(())`, INSERTs a row | row exists post-call; recorder saw `ReducerBegin` + `ReducerCommit` with the fn name |
| `error_rollbacks` | body INSERTs then returns `Err(...)` | row absent; recorder saw `ReducerBegin` + `ReducerRollback` with stringified error |
| `panic_rollbacks_and_resumes` | body INSERTs then `panic!()` | `std::panic::catch_unwind` at the test level catches the resumed panic; row absent; recorder saw `ReducerRollback` with `error: "panic"` |
| `serializable_isolation_applied` | `#[reducer(isolation = "serializable")]` body in two concurrent tasks racing on the same row | one task sees PG `40001` (serialization failure); test asserts which |

### 7.3 Coverage handling

The Coverage workflow's existing ignore-filename-regex (`.github/workflows/codecov.yml`) already excludes `sntl-macros/` as a whole, so the new `sntl-macros/src/reducer/` directory is covered by that exclusion automatically — no workflow change required.

The existing `core::observability::tests::synthetic_all_arms_execute` test (in `sntl/src/core/observability.rs`) already exercises the `ReducerBegin/Commit/Rollback` arms via a synthetic recorder. It stays as-is — no changes needed.

---

## 8. Documentation Surface

### 8.1 New file

`docs/reducer-guide.md` (~500 words), structured like `docs/testing-guide.md`:

1. **30-sec quickstart** — minimal `#[sntl::reducer]` example.
2. **Three forms** — default / with isolation / panic-safe.
3. **Observability** — link to `docs/observability-guide.md`; show what each event looks like in tracing output.
4. **Limitations (v0.6):**
   - First arg must be `&mut Connection` — for `&pool` callers, acquire first.
   - Nested reducers not supported — `BEGIN` inside a transaction warns on PG; results undefined.
   - No savepoints — for partial rollback, use explicit `Transaction::begin` + `SAVEPOINT` SQL.
   - Pool poisoning on rollback-after-panic — tracked for v0.7.
   - Requires `unsafe` in expansion — incompatible with user crates that set `#![forbid(unsafe_code)]`; document `Transaction` fallback.

### 8.2 Updates to existing docs

- **`README.md` Features list** — append bullet: "**Transactional reducers** — `#[sntl::reducer]` wraps any async fn in BEGIN/COMMIT/ROLLBACK with isolation control and observability events. See [`docs/reducer-guide.md`](docs/reducer-guide.md)."
- **`sntl/src/lib.rs` crate-level doc** — extend the "Day-one DX" callout with a `#[sntl::reducer]` line and a link to the guide.
- **`docs/migration-from-sqlx.md`** — new section "Replacing `sqlx::Transaction` with `#[sntl::reducer]`": one diff example replacing `let mut tx = pool.begin().await?;` + manual `tx.commit()` with the attribute form.
- **`docs/observability-guide.md`** — strike the "Reducer events dormant" line from the Limitations table (or update it to "fires from `#[sntl::reducer]`-wrapped fns").
- **`CLAUDE.md`** — update the "Key Patterns" reducer entry from a forward-looking description to a present-tense one with a code pointer.
- **memory `roadmap_sentinel.md`** — append a v0.6 section noting `#[reducer]` shipped (closes the largest gap identified by `/graphify` audit, 2026-05-22).

### 8.3 changelog

`sntl-macros` and `sntl` both bump to 0.1.3. Changelog entries auto-generated by release-please from the `feat(sntl-macros): #[reducer]` commit subject.

---

## 9. Open Items (NOT in MVP, tracked for v0.7+)

| Item | Why deferred |
|---|---|
| Auto-reorder lock by ID (deadlock prevention) | Not a macro feature — needs runtime lock manager. Separate spec `LockBatch` API. |
| `&Pool` first-arg form (auto-acquire) | Two-line workaround; v0.6 keeps macro shape focused. |
| Nested reducers via savepoints | Requires savepoint name management + driver `SAVEPOINT` API. |
| Pool poisoning on rollback failure | Driver-side change — needs `Pool::recycle_on_error` or similar. |
| `compile_error!` for missing `E: Display` | Friendlier diagnostic; rustc message is workable for v0.6. |
| Custom event names via `#[reducer(name = "...")]` | Not in §1-6 of this design; defer until a user asks. |

---

## 10. Verification Checklist

- [x] Architecture matches `sntl-macros/src/test/` precedent — yes, same `mod.rs` + `codegen.rs` layout.
- [x] Driver `Event` arms already declared — `instrumentation.rs:120-130` (driver v4.0.0).
- [x] `SntlTracing` handler arms already wired — `sntl/src/core/observability.rs:37-50`.
- [x] `Connection::instrumentation()` accessor exists — driver `connection/client.rs:219`.
- [x] `Transaction::begin_with` + `IsolationLevel` enum exist — `sntl/src/core/transaction.rs:29-39`.
- [x] No new driver dependency needed — all hooks already shipped in v4.0.0.
- [x] Scope tight enough for single implementation plan — yes, ~6 tasks.
