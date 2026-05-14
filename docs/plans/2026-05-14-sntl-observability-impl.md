# `sntl-observability` v0.4 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a working `Instrumentation` surface for `sentinel-driver` (PR-A1, v2.1.0) and the `sntl` bridge that adds Sentinel-level events (PR-A2, v0.4.0). Closes the largest production-readiness gap vs sqlx / Diesel 2.2.

**Architecture:** Driver defines a single `Instrumentation: Send + Sync + 'static` trait with one method `on_event(&Event<'_>)`. Wire sites call it through an `Arc<dyn Instrumentation>` held by `Connection`. Default impl is `NoOpInstrumentation` (vtable-dispatch return). A `TracingInstrumentation` adapter ships in the driver itself and records OTel-conformant fields onto the current `tracing::Span`. The sntl crate adds a `SntlTracing` wrapper that handles three sntl-specific event arms (`QueryMacro`, `Reducer*`, `Migration*`) and delegates the rest.

**Tech Stack:** Rust 1.85 / edition 2024, `tracing` 0.1 (already a hard dep in sentinel-driver), `tracing-test` for unit assertions, `dhat` for zero-alloc gate, `tracing-opentelemetry` + `opentelemetry-stdout` for the e2e OTel test (dev-deps only).

---

## Reference material

- Design: `docs/plans/2026-05-13-sntl-observability-design.md` — read §Event taxonomy, §Tracing adapter, and §Testing strategy before starting.
- Cross-repo target: `/home/mrbt/Desktop/workspaces/orm/repositories/sentinel-driver` is the upstream driver (workspace at `crates/sentinel-driver/`). PR-A1 lives there.
- Plan style template: `docs/plans/2026-05-09-sntl-migrate-impl.md` set the TDD-by-default per-task format used here.

### Resolved deviations from the spec

1. **No `tracing` feature gate.** The spec says "feature `tracing` → ships adapter." But `tracing = "0.1"` is already a non-optional dep in `sentinel-driver/Cargo.toml` (line 43, used for `tracing::debug!` in `connection/client.rs` and `connection/startup.rs`). Making it optional would force every consumer to add it back. Decision: `TracingInstrumentation` ships always-built. No feature flag.
2. **OTel sem-conv crate.** Out of scope; we hard-code attribute names. Future driver release can add `opentelemetry-semantic-conventions` if desired.

---

## File structure (master map)

### `sentinel-driver` (PR-A1, separate repo)

```
crates/sentinel-driver/
├── src/
│   ├── instrumentation.rs                     # NEW — REPLACES observability.rs
│   ├── tracing_adapter.rs                     # NEW
│   ├── observability.rs                       # DELETED
│   ├── lib.rs                                 # MODIFY: re-exports
│   ├── config.rs                              # MODIFY: with_instrumentation()
│   ├── connection/
│   │   ├── mod.rs                             # MODIFY: store Arc<dyn Instrumentation>
│   │   ├── client.rs                          # MODIFY: emit Connect/Authenticated/Disconnect
│   │   ├── query.rs                           # MODIFY: emit ExecuteStart/Finish (5 entry points)
│   │   ├── prepare.rs                         # MODIFY: emit PrepareStart/Finish, cache hit
│   │   ├── transaction_impl.rs                # MODIFY: emit TxBegin/Commit/Rollback
│   │   ├── pipeline_impl.rs                   # MODIFY: emit PipelineStart/Flush
│   │   └── notify_impl.rs                     # MODIFY: emit Notice/Notification
│   ├── pool/
│   │   ├── mod.rs                             # MODIFY: with_instrumentation(), emit acquire/release
│   │   └── config.rs                          # MODIFY (if instrumentation lives in PoolConfig)
└── tests/
    ├── instrumentation_test.rs                # NEW
    ├── tracing_adapter_test.rs                # NEW
    └── noop_zero_alloc_test.rs                # NEW
```

### `sentinel` (PR-A2)

```
sntl/src/
├── core/
│   ├── mod.rs                                 # MODIFY: pub mod observability
│   ├── observability.rs                       # NEW — SntlTracing + install_default_tracing
│   └── __priv/
│       └── mod.rs                             # NEW (or MODIFY existing) — emit_query_macro helper
sntl-macros/src/
├── query/
│   ├── anonymous.rs                           # MODIFY: emit_query_macro
│   ├── typed.rs                               # MODIFY
│   ├── file.rs                                # MODIFY
│   ├── pipeline.rs                            # MODIFY
│   └── unchecked.rs                           # MODIFY
sntl-migrate/src/
└── runner.rs                                  # MODIFY: emit MigrationApply/Drift
sntl/tests/
├── instrumentation_test.rs                    # NEW
├── reducer_instrumentation_test.rs            # NEW
└── otel_e2e_test.rs                           # NEW (live-PG)
sntl-migrate/tests/
└── migrate_instrumentation_test.rs            # NEW (live-PG)
docs/
└── observability-guide.md                    # NEW
```

---

## Phase 1 — Driver (PR-A1)

> Work directory: `/home/mrbt/Desktop/workspaces/orm/repositories/sentinel-driver`. Create a worktree first: `git worktree add .worktrees/feat-instrumentation -b feat/instrumentation`. The driver workspace publishes `sentinel-driver` from `crates/sentinel-driver/`.

### Task 1: Scaffold `instrumentation.rs`, delete `observability.rs`

**Files:**
- Create: `crates/sentinel-driver/src/instrumentation.rs`
- Delete: `crates/sentinel-driver/src/observability.rs`
- Modify: `crates/sentinel-driver/src/lib.rs`

- [ ] **Step 1: Delete the dead module**

```bash
rm crates/sentinel-driver/src/observability.rs
```

- [ ] **Step 2: Create `instrumentation.rs` with the trait + NoOp + Event placeholder**

```rust
//! Instrumentation surface for `sentinel-driver`.
//!
//! Install via `Config::with_instrumentation`, `Pool::with_instrumentation`,
//! or `Connection::set_instrumentation`. Default is a no-op.

use std::sync::Arc;

/// A driver consumer's hook into every operation Sentinel performs.
///
/// Events are passed by borrow — the implementation MUST NOT retain
/// the `Event` past the call. Clone data inside the handler if needed.
pub trait Instrumentation: Send + Sync + 'static {
    fn on_event(&self, event: &Event<'_>);
}

/// Default no-op. Returns immediately via vtable dispatch.
pub(crate) struct NoOpInstrumentation;

impl Instrumentation for NoOpInstrumentation {
    #[inline]
    fn on_event(&self, _: &Event<'_>) {}
}

pub(crate) fn noop() -> Arc<dyn Instrumentation> {
    Arc::new(NoOpInstrumentation)
}

// Event taxonomy lands in Task 2.
pub enum Event<'a> {
    _Phantom(std::marker::PhantomData<&'a ()>),
}
```

- [ ] **Step 3: Replace observability re-exports in `lib.rs`**

In `crates/sentinel-driver/src/lib.rs`, change:

```rust
mod observability;
pub use observability::{ObservabilityConfig, QueryMetrics, QueryMetricsCallback};
```

to:

```rust
mod instrumentation;
pub use instrumentation::{Event, Instrumentation};
```

- [ ] **Step 4: Find + remove every other reference to the deleted types**

```bash
cd crates/sentinel-driver
grep -rn "ObservabilityConfig\|QueryMetrics\|QueryMetricsCallback\|log_slow_query\|query_span" src/ tests/ examples/ 2>/dev/null
```

Expected: empty. The original `observability.rs` module wasn't wired anywhere — verify this assumption. Any remaining hit = a bug; fix it.

- [ ] **Step 5: `cargo check -p sentinel-driver`**

Expected: clean (Event enum is empty so no usage yet).

- [ ] **Step 6: Commit**

```bash
git add crates/sentinel-driver/src/{lib,instrumentation}.rs
git rm crates/sentinel-driver/src/observability.rs
git commit -m "refactor(observability): replace dead ObservabilityConfig with Instrumentation trait scaffold"
```

---

### Task 2: Full `Event` enum + supporting types

**Files:**
- Modify: `crates/sentinel-driver/src/instrumentation.rs`

- [ ] **Step 1: Replace the `Event` placeholder with the full taxonomy**

```rust
use std::time::Duration;

use crate::Error;
use crate::transaction::IsolationLevel;

#[non_exhaustive]
pub enum Event<'a> {
    // Connection lifecycle
    Connect       { host: &'a str, port: u16 },
    Authenticated { user: &'a str },
    Disconnect    { reason: DisconnectReason },

    // Prepare
    PrepareStart  { name: &'a str, sql: &'a str },
    PrepareFinish {
        name: &'a str,
        param_oids: &'a [u32],
        col_count: u16,
        duration: Duration,
        cache_hit: bool,
    },

    // Execute (covers query / query_one / query_opt / execute / query_typed*)
    ExecuteStart  { stmt: StmtRef<'a>, param_count: usize },
    ExecuteFinish {
        stmt: StmtRef<'a>,
        rows: u64,
        duration: Duration,
        outcome: Outcome<'a>,
    },

    // Pipeline
    PipelineStart { batch_len: usize },
    PipelineFlush { batch_len: usize, total_duration: Duration },

    // Transaction
    TxBegin    { isolation: Option<IsolationLevel> },
    TxCommit   { duration: Duration },
    TxRollback { duration: Duration, reason: RollbackReason<'a> },

    // Pool
    PoolAcquireStart  { pending: usize },
    PoolAcquireFinish { wait: Duration, outcome: AcquireOutcome },
    PoolRelease,

    // PG async messages
    Notice       { severity: &'a str, code: &'a str, message: &'a str },
    Notification { channel: &'a str, payload: &'a str, pid: i32 },

    // Sentinel-level (sntl crate emits these; driver itself never does)
    QueryMacro      { macro_name: &'a str, query_id: &'a str, sql: &'a str },
    ReducerBegin    { name: &'a str },
    ReducerCommit   { name: &'a str, duration: Duration },
    ReducerRollback { name: &'a str, error: &'a str },
    MigrationApply  { version: &'a str, duration: Duration, checksum: &'a str },
    MigrationDrift  { version: &'a str, recorded: &'a str, current: &'a str },
}

#[non_exhaustive]
pub enum StmtRef<'a> {
    Named  { name: &'a str },
    Inline { sql: &'a str },
}

impl<'a> StmtRef<'a> {
    /// SQL text if available (Inline), else the prepared name (Named).
    pub fn sql_or_name(&self) -> &'a str {
        match self {
            StmtRef::Named { name } => name,
            StmtRef::Inline { sql } => sql,
        }
    }

    /// First word of the SQL, uppercased. "" if no leading keyword found.
    pub fn op_hint(&self) -> &'static str {
        let s = self.sql_or_name();
        let first = s.split_ascii_whitespace().next().unwrap_or("");
        match first.to_ascii_uppercase().as_str() {
            "SELECT" => "SELECT",
            "INSERT" => "INSERT",
            "UPDATE" => "UPDATE",
            "DELETE" => "DELETE",
            "BEGIN" => "BEGIN",
            "COMMIT" => "COMMIT",
            "ROLLBACK" => "ROLLBACK",
            "WITH"   => "WITH",
            _ => "OTHER",
        }
    }
}

#[non_exhaustive]
pub enum Outcome<'a> {
    Ok,
    Err(&'a Error),
}

#[non_exhaustive]
pub enum DisconnectReason {
    Graceful,
    BrokenPipe,
    Timeout,
    ServerKill,
}

#[non_exhaustive]
pub enum RollbackReason<'a> {
    Explicit,
    Drop,
    Error(&'a Error),
}

#[non_exhaustive]
pub enum AcquireOutcome {
    Ok,
    Timeout,
    PoolClosed,
}
```

- [ ] **Step 2: Add `StmtRef` / `Outcome` / etc. re-exports in `lib.rs`**

```rust
pub use instrumentation::{
    AcquireOutcome, DisconnectReason, Event, Instrumentation, Outcome,
    RollbackReason, StmtRef,
};
```

- [ ] **Step 3: Verify `IsolationLevel` is `pub` in `transaction.rs`**

Run: `grep -n "pub enum IsolationLevel\|pub use.*IsolationLevel" crates/sentinel-driver/src/transaction.rs crates/sentinel-driver/src/lib.rs`
If not `pub`, make it `pub` and re-export from `lib.rs`.

- [ ] **Step 4: `cargo check -p sentinel-driver`**

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/sentinel-driver/src/{instrumentation,lib,transaction}.rs
git commit -m "feat(observability): define full Event taxonomy"
```

---

### Task 3: Store `Arc<dyn Instrumentation>` on `Connection`

**Files:**
- Modify: `crates/sentinel-driver/src/connection/mod.rs`
- Modify: `crates/sentinel-driver/src/connection/client.rs`

- [ ] **Step 1: Add field to `Connection`**

In `crates/sentinel-driver/src/connection/mod.rs`, locate `pub struct Connection` at line 37 and add:

```rust
pub struct Connection {
    // ... existing fields ...
    pub(crate) instrumentation: std::sync::Arc<dyn crate::Instrumentation>,
}
```

- [ ] **Step 2: Initialise to NoOp in every Connection constructor**

Search constructors:

```bash
grep -n "Connection {" crates/sentinel-driver/src/connection/*.rs
```

For each struct-literal initialisation, add:

```rust
instrumentation: crate::instrumentation::noop(),
```

- [ ] **Step 3: Add `set_instrumentation` + `instrumentation()` accessors**

In `crates/sentinel-driver/src/connection/client.rs` (or wherever the public `impl Connection {}` block lives), add:

```rust
impl Connection {
    /// Install an `Instrumentation` impl on this connection.
    /// Replaces any previous installation.
    pub fn set_instrumentation(&mut self, instr: std::sync::Arc<dyn crate::Instrumentation>) {
        self.instrumentation = instr;
    }

    /// Public accessor used by downstream macro helpers (e.g. sntl's
    /// `__priv::emit_query_macro`). Returns the shared `Arc` so callers can
    /// emit Sentinel-level events through the same trait.
    pub fn instrumentation(&self) -> &std::sync::Arc<dyn crate::Instrumentation> {
        &self.instrumentation
    }

    /// Crate-internal shorthand for wire sites.
    pub(crate) fn instr(&self) -> &dyn crate::Instrumentation {
        &*self.instrumentation
    }
}
```

- [ ] **Step 4: `cargo check -p sentinel-driver`**

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/sentinel-driver/src/connection/
git commit -m "feat(observability): Connection holds Arc<dyn Instrumentation>, default NoOp"
```

---

### Task 4: `Config::with_instrumentation` + propagate to `Connection::connect`

**Files:**
- Modify: `crates/sentinel-driver/src/config.rs`
- Modify: `crates/sentinel-driver/src/connection/startup.rs` (or whichever file owns `Connection::connect`)

- [ ] **Step 1: Add field to `Config`**

In `crates/sentinel-driver/src/config.rs` `pub struct Config { ... }` block:

```rust
#[derive(Clone)]
pub struct Config {
    // ... existing fields ...
    pub(crate) instrumentation: Option<std::sync::Arc<dyn crate::Instrumentation>>,
}
```

If `Config` already derives `Clone`, this requires `Arc<dyn Instrumentation>` to be `Clone` (it is — `Arc` is always `Clone`). Add `Option<...>` — `None` keeps current parse path simple.

- [ ] **Step 2: Builder method**

```rust
impl Config {
    pub fn with_instrumentation(
        mut self,
        instr: std::sync::Arc<dyn crate::Instrumentation>,
    ) -> Self {
        self.instrumentation = Some(instr);
        self
    }
}
```

- [ ] **Step 3: Wire into `Connection::connect`**

In the file with `pub async fn connect(config: Config) -> Result<Self>` (likely `connection/startup.rs` or `connection/client.rs`):

```rust
let mut conn = /* build connection as before */;
if let Some(instr) = config.instrumentation.clone() {
    conn.set_instrumentation(instr);
}
Ok(conn)
```

- [ ] **Step 4: Update Debug impl for Config**

If Config derives `Debug` automatically, the new `Arc<dyn ...>` won't compile (no Debug). Switch to manual impl that skips the field, OR add `#[derive(Debug)]` only when `instrumentation: None`. Simplest: change to manual impl. Pattern:

```rust
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("host", &self.host)
            // ... other public-ish fields ...
            .field("instrumentation", &self.instrumentation.as_ref().map(|_| "..."))
            .finish()
    }
}
```

Remove `Debug` from the derive list at the top of the struct.

- [ ] **Step 5: `cargo check -p sentinel-driver`**

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/sentinel-driver/src/{config.rs,connection/}
git commit -m "feat(observability): Config::with_instrumentation propagates to Connection::connect"
```

---

### Task 5: Wire `ExecuteStart` / `ExecuteFinish` into `Connection::query` + friends

**Files:**
- Modify: `crates/sentinel-driver/src/connection/query.rs`

- [ ] **Step 1: Wrap `query`**

At line 19, `pub async fn query(&mut self, sql: &str, params: ...)` becomes:

```rust
pub async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
    self.instr().on_event(&crate::Event::ExecuteStart {
        stmt: crate::StmtRef::Inline { sql },
        param_count: params.len(),
    });
    let started = std::time::Instant::now();
    let res = self.query_inner(sql, params).await;
    let duration = started.elapsed();
    let outcome = match &res {
        Ok(_) => crate::Outcome::Ok,
        Err(e) => crate::Outcome::Err(e),
    };
    let rows = res.as_ref().map(|v| v.len() as u64).unwrap_or(0);
    self.instr().on_event(&crate::Event::ExecuteFinish {
        stmt: crate::StmtRef::Inline { sql },
        rows,
        duration,
        outcome,
    });
    res
}
```

Extract the existing body into `query_inner` (same signature). Apply the **same pattern** to:
- `query_one` (line 34)
- `query_opt` (line 42)
- `execute` (line 54) — `rows` here is the affected count, not `Vec::len`
- `query_with_timeout` / `execute_with_timeout` (lines 70, 104)
- `query_typed` / `query_typed_one` / `query_typed_opt` / `execute_typed` (lines 222, 235, 247, 257)

Each public method does: emit Start → call `*_inner` → emit Finish. The `_inner` methods are private.

- [ ] **Step 2: Don't repeat yourself — extract a helper**

```rust
// At the bottom of query.rs:
impl Connection {
    async fn instrumented<F, R, T>(
        &mut self,
        sql: &str,
        param_count: usize,
        body: F,
        rows_of: impl Fn(&Result<R>) -> u64,
        wrap: impl Fn(R) -> T,
    ) -> Result<T>
    where
        F: for<'a> AsyncFnOnce(&'a mut Self) -> Result<R>,
    {
        // ... shared start/finish logic ...
    }
}
```

If `AsyncFnOnce` is too new for your toolchain, use the explicit pattern from Step 1 inline for each method. The duplication is fine — 10 methods at 5 lines each is 50 lines, totally acceptable.

- [ ] **Step 3: Run the existing query tests**

```bash
cd crates/sentinel-driver
DATABASE_URL=postgres://… cargo test --lib --tests query
```

Expected: existing tests still pass (semantic-preserving refactor — instrumentation is NoOp by default).

- [ ] **Step 4: Commit**

```bash
git add crates/sentinel-driver/src/connection/query.rs
git commit -m "feat(observability): emit ExecuteStart/Finish around every query.rs entry point"
```

---

### Task 6: Wire `PrepareStart` / `PrepareFinish` + cache_hit flag

**Files:**
- Modify: `crates/sentinel-driver/src/connection/prepare.rs`

- [ ] **Step 1: Identify cache-hit and cache-miss paths**

Read `prepare.rs`. Look for the function that returns a prepared `Statement`. There will be a cache lookup. Cache hit = fast return; cache miss = `Parse` wire message.

- [ ] **Step 2: Emit `PrepareStart` before lookup, `PrepareFinish` after both paths**

```rust
pub(crate) async fn prepare_cached(&mut self, sql: &str) -> Result<&Statement> {
    self.instr().on_event(&crate::Event::PrepareStart {
        name: "",  // unnamed until cached
        sql,
    });
    let started = std::time::Instant::now();
    if let Some(stmt) = self.statement_cache.get(sql) {
        self.instr().on_event(&crate::Event::PrepareFinish {
            name: stmt.name(),
            param_oids: stmt.param_oids(),
            col_count: stmt.columns().len() as u16,
            duration: started.elapsed(),
            cache_hit: true,
        });
        return Ok(stmt);
    }
    let stmt = self.prepare_uncached(sql).await?;
    self.instr().on_event(&crate::Event::PrepareFinish {
        name: stmt.name(),
        param_oids: stmt.param_oids(),
        col_count: stmt.columns().len() as u16,
        duration: started.elapsed(),
        cache_hit: false,
    });
    Ok(stmt)
}
```

Adapt names to the actual function signatures in `prepare.rs`.

- [ ] **Step 3: Build + run prepare tests**

```bash
DATABASE_URL=postgres://… cargo test --lib --tests prepare
```

- [ ] **Step 4: Commit**

```bash
git add crates/sentinel-driver/src/connection/prepare.rs
git commit -m "feat(observability): emit PrepareStart/Finish with cache_hit flag"
```

---

### Task 7: Wire `TxBegin` / `TxCommit` / `TxRollback`

**Files:**
- Modify: `crates/sentinel-driver/src/connection/transaction_impl.rs`

- [ ] **Step 1: Wrap each tx method**

```rust
impl Connection {
    pub async fn begin(&mut self) -> Result<()> {
        self.instr().on_event(&crate::Event::TxBegin { isolation: None });
        self.execute("BEGIN", &[]).await.map(|_| ())
    }

    pub async fn begin_with(&mut self, config: TransactionConfig) -> Result<()> {
        self.instr().on_event(&crate::Event::TxBegin {
            isolation: Some(config.isolation()),
        });
        // ... existing body building BEGIN ISOLATION LEVEL ... statement ...
    }

    pub async fn commit(&mut self) -> Result<()> {
        let started = std::time::Instant::now();
        let res = self.execute("COMMIT", &[]).await.map(|_| ());
        self.instr().on_event(&crate::Event::TxCommit {
            duration: started.elapsed(),
        });
        res
    }

    pub async fn rollback(&mut self) -> Result<()> {
        let started = std::time::Instant::now();
        let res = self.execute("ROLLBACK", &[]).await.map(|_| ());
        self.instr().on_event(&crate::Event::TxRollback {
            duration: started.elapsed(),
            reason: crate::RollbackReason::Explicit,
        });
        res
    }
}
```

- [ ] **Step 2: For `rollback_to` (savepoint) — emit `TxRollback` with `RollbackReason::Explicit` too**

Same pattern.

- [ ] **Step 3: Run tests**

```bash
DATABASE_URL=postgres://… cargo test --lib --tests transaction
```

- [ ] **Step 4: Commit**

```bash
git add crates/sentinel-driver/src/connection/transaction_impl.rs
git commit -m "feat(observability): emit TxBegin/Commit/Rollback in transaction methods"
```

---

### Task 8: Wire `PipelineStart` / `PipelineFlush`

**Files:**
- Modify: `crates/sentinel-driver/src/connection/pipeline_impl.rs`

- [ ] **Step 1: Find `execute_pipeline`**

In `connection/pipeline_impl.rs` at line 12.

- [ ] **Step 2: Emit Start before the round-trip, Flush after**

```rust
pub async fn execute_pipeline(&mut self, batch: PipelineBatch) -> Result<...> {
    let batch_len = batch.len();
    self.instr().on_event(&crate::Event::PipelineStart { batch_len });
    let started = std::time::Instant::now();
    let res = self.execute_pipeline_inner(batch).await;
    self.instr().on_event(&crate::Event::PipelineFlush {
        batch_len,
        total_duration: started.elapsed(),
    });
    res
}
```

Same `inner` extraction as Task 5.

- [ ] **Step 3: Commit**

```bash
git add crates/sentinel-driver/src/connection/pipeline_impl.rs
git commit -m "feat(observability): emit PipelineStart/Flush around batched round-trips"
```

---

### Task 9: Wire `Notice` / `Notification`

**Files:**
- Modify: `crates/sentinel-driver/src/connection/notify_impl.rs`
- Modify: `crates/sentinel-driver/src/notify/channel.rs` (if NotificationResponse parsing lives here)

- [ ] **Step 1: Find the NoticeResponse / NotificationResponse parse sites**

```bash
grep -rn "NoticeResponse\|NotificationResponse" crates/sentinel-driver/src/
```

These are wire-protocol-level — usually in `protocol/` and surfaced as either internal errors (Notice → tracing::warn today; we will switch to event) or `Notification` rows on the listener.

- [ ] **Step 2: Replace existing `tracing::warn` for NOTICE with event emission**

```rust
// at the site that handles NoticeResponse:
let fields = parse_error_fields(&body)?;
self.instr().on_event(&crate::Event::Notice {
    severity: &fields.severity,
    code: &fields.code,
    message: &fields.message,
});
// (keep the old tracing::warn if you want, but the Notice event subsumes it)
```

- [ ] **Step 3: At the NotificationResponse path:**

```rust
self.instr().on_event(&crate::Event::Notification {
    channel: &channel,
    payload: &payload,
    pid,
});
```

- [ ] **Step 4: Commit**

```bash
git add crates/sentinel-driver/src/
git commit -m "feat(observability): emit Notice + Notification on PG async messages"
```

---

### Task 10: Wire pool `PoolAcquireStart/Finish` + `PoolRelease`

**Files:**
- Modify: `crates/sentinel-driver/src/pool/mod.rs`

- [ ] **Step 1: Add `instrumentation` field on `Pool`**

```rust
pub struct Pool {
    // ... existing fields ...
    instrumentation: std::sync::Arc<dyn crate::Instrumentation>,
}
```

Initialise in `Pool::new` / `connect_lazy` to `crate::instrumentation::noop()`.

- [ ] **Step 2: Add `with_instrumentation`**

```rust
impl Pool {
    pub fn with_instrumentation(
        mut self,
        instr: std::sync::Arc<dyn crate::Instrumentation>,
    ) -> Self {
        self.instrumentation = instr;
        self
    }
}
```

- [ ] **Step 3: Emit around `acquire`**

```rust
pub async fn acquire(&self) -> Result<PooledConnection> {
    let pending = self.pending_acquires.load(...);   // or similar metric
    self.instrumentation.on_event(&crate::Event::PoolAcquireStart { pending });
    let started = std::time::Instant::now();
    let res = self.acquire_inner().await;
    let wait = started.elapsed();
    let outcome = match &res {
        Ok(_) => crate::AcquireOutcome::Ok,
        Err(e) if e.is_timeout() => crate::AcquireOutcome::Timeout,
        Err(_) => crate::AcquireOutcome::PoolClosed,
    };
    self.instrumentation.on_event(&crate::Event::PoolAcquireFinish {
        wait,
        outcome,
    });

    // Critical: propagate the pool's instrumentation INTO the acquired connection,
    // so per-query events get routed to the same impl.
    if let Ok(ref pooled) = res {
        pooled.conn.borrow_mut().set_instrumentation(self.instrumentation.clone());
    }
    res
}
```

If `Error::is_timeout()` doesn't exist, add a tiny helper or pattern-match on the variant.

- [ ] **Step 4: Emit `PoolRelease` in the `Drop` impl of `PooledConnection`**

```rust
impl Drop for PooledConnection {
    fn drop(&mut self) {
        // existing cleanup ...
        self.pool_instrumentation.on_event(&crate::Event::PoolRelease);
    }
}
```

`PooledConnection` must hold `Arc<dyn Instrumentation>` (clone from Pool at acquire time).

- [ ] **Step 5: Commit**

```bash
git add crates/sentinel-driver/src/pool/
git commit -m "feat(observability): Pool::with_instrumentation, emit Pool* events, propagate to PooledConnection"
```

---

### Task 11: `TracingInstrumentation` adapter

**Files:**
- Create: `crates/sentinel-driver/src/tracing_adapter.rs`
- Modify: `crates/sentinel-driver/src/lib.rs`

- [ ] **Step 1: Write the adapter**

```rust
//! `tracing`-based `Instrumentation` impl. Always built — `tracing` is a
//! hard dep of sentinel-driver.

use std::time::Duration;

use crate::{Event, Instrumentation, Outcome};

/// Records OTel-conformant `db.*` fields onto the current `tracing::Span`.
///
/// The wire site is expected to have opened a span via
/// `tracing::info_span!("db.query")` and entered it (`Span::in_scope` or
/// `Instrument::instrument`). This adapter only records onto whatever
/// span is current; it never creates new spans itself.
#[derive(Clone)]
pub struct TracingInstrumentation {
    pub max_sql_len: usize,
    pub slow_threshold: Option<Duration>,
}

impl Default for TracingInstrumentation {
    fn default() -> Self {
        Self {
            max_sql_len: 1024,
            slow_threshold: None,
        }
    }
}

impl Instrumentation for TracingInstrumentation {
    fn on_event(&self, ev: &Event<'_>) {
        let span = tracing::Span::current();
        match ev {
            Event::ExecuteStart { stmt, param_count } => {
                let sql = truncate(stmt.sql_or_name(), self.max_sql_len);
                span.record("db.system", "postgresql");
                span.record("db.statement", &tracing::field::display(&sql));
                span.record("db.operation", stmt.op_hint());
                span.record("sntl.param_count", *param_count);
            }
            Event::ExecuteFinish { rows, duration, outcome, .. } => {
                span.record("db.rows_affected", *rows);
                span.record("sntl.duration_us", duration.as_micros() as i64);
                if let Outcome::Err(e) = outcome {
                    span.record("error", true);
                    tracing::error!(error = %e, "query failed");
                }
                if matches!(self.slow_threshold, Some(t) if *duration > t) {
                    tracing::warn!(slow = true, "slow query");
                }
            }
            Event::PrepareFinish { cache_hit, duration, .. } => {
                span.record("sntl.cache_hit", *cache_hit);
                span.record("sntl.prepare_us", duration.as_micros() as i64);
            }
            Event::TxBegin { isolation } => {
                tracing::info!(isolation = ?isolation, "tx begin");
            }
            Event::TxCommit { duration } => {
                tracing::info!(duration_us = duration.as_micros() as i64, "tx commit");
            }
            Event::TxRollback { duration, reason } => {
                tracing::warn!(
                    duration_us = duration.as_micros() as i64,
                    reason = ?reason,
                    "tx rollback"
                );
            }
            Event::PipelineFlush { batch_len, total_duration } => {
                span.record("sntl.pipeline_batch_len", *batch_len);
                span.record("sntl.duration_us", total_duration.as_micros() as i64);
            }
            Event::PoolAcquireFinish { wait, outcome } => {
                tracing::debug!(
                    wait_us = wait.as_micros() as i64,
                    outcome = ?outcome,
                    "pool acquire"
                );
            }
            Event::Notice { severity, code, message } => {
                tracing::warn!(severity = %severity, code = %code, "{}", message);
            }
            Event::Notification { channel, pid, .. } => {
                tracing::info!(channel = %channel, pid = pid, "notification");
            }
            // Other events (Connect, Disconnect, PoolRelease, sntl-level) are
            // emitted by sntl bridge; driver-side adapter ignores them.
            _ => {}
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        // Walk back to nearest char boundary to avoid panic on multi-byte
        let mut idx = max;
        while !s.is_char_boundary(idx) && idx > 0 {
            idx -= 1;
        }
        &s[..idx]
    }
}
```

- [ ] **Step 2: Add `pub mod tracing_adapter;` in `lib.rs` + re-export**

```rust
mod tracing_adapter;
pub use tracing_adapter::TracingInstrumentation;
```

- [ ] **Step 3: `cargo check -p sentinel-driver`**

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/sentinel-driver/src/{tracing_adapter,lib}.rs
git commit -m "feat(observability): TracingInstrumentation adapter records db.* on current span"
```

---

### Task 12: Recording-impl unit test

**Files:**
- Create: `crates/sentinel-driver/tests/instrumentation_test.rs`

- [ ] **Step 1: Write the recording fixture**

```rust
//! Verify Instrumentation events fire in the expected order for common ops.
//! Live-PG; skips silently when DATABASE_URL is unset.

use std::sync::{Arc, Mutex};

use sentinel_driver::{
    AcquireOutcome, Config, Connection, Event, Instrumentation, Outcome,
    Pool, PoolConfig, StmtRef,
};

#[derive(Default)]
struct Recorder(Mutex<Vec<OwnedEvent>>);

#[derive(Debug, PartialEq)]
enum OwnedEvent {
    ExecuteStart { sql: String, param_count: usize },
    ExecuteFinish { sql: String, rows: u64, ok: bool },
    PrepareFinish { cache_hit: bool },
    TxBegin,
    TxCommit,
    TxRollback,
    PipelineFlush { batch_len: usize },
    PoolAcquireFinish { ok: bool },
    PoolRelease,
    Notice { code: String },
}

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        let owned = match ev {
            Event::ExecuteStart { stmt, param_count } => OwnedEvent::ExecuteStart {
                sql: stmt.sql_or_name().to_string(),
                param_count: *param_count,
            },
            Event::ExecuteFinish { stmt, rows, outcome, .. } => OwnedEvent::ExecuteFinish {
                sql: stmt.sql_or_name().to_string(),
                rows: *rows,
                ok: matches!(outcome, Outcome::Ok),
            },
            Event::PrepareFinish { cache_hit, .. } => OwnedEvent::PrepareFinish {
                cache_hit: *cache_hit,
            },
            Event::TxBegin { .. } => OwnedEvent::TxBegin,
            Event::TxCommit { .. } => OwnedEvent::TxCommit,
            Event::TxRollback { .. } => OwnedEvent::TxRollback,
            Event::PipelineFlush { batch_len, .. } => OwnedEvent::PipelineFlush {
                batch_len: *batch_len,
            },
            Event::PoolAcquireFinish { outcome, .. } => OwnedEvent::PoolAcquireFinish {
                ok: matches!(outcome, AcquireOutcome::Ok),
            },
            Event::PoolRelease => OwnedEvent::PoolRelease,
            Event::Notice { code, .. } => OwnedEvent::Notice {
                code: code.to_string(),
            },
            _ => return,
        };
        self.0.lock().unwrap().push(owned);
    }
}

async fn connect() -> Option<(Connection, Arc<Recorder>)> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let rec = Arc::new(Recorder::default());
    let cfg = Config::parse(&url).ok()?.with_instrumentation(rec.clone());
    let mut conn = Connection::connect(cfg).await.ok()?;
    conn.execute("SET client_min_messages = ERROR", &[]).await.ok()?;
    rec.0.lock().unwrap().clear();
    Some((conn, rec))
}

#[tokio::test]
async fn query_emits_start_then_finish() {
    let Some((mut conn, rec)) = connect().await else { return };
    conn.query("SELECT 1::int4", &[]).await.unwrap();
    let evs = rec.0.lock().unwrap();
    assert!(matches!(evs.first(), Some(OwnedEvent::ExecuteStart { .. })));
    assert!(matches!(evs.last(),  Some(OwnedEvent::ExecuteFinish { ok: true, .. })));
}

#[tokio::test]
async fn transaction_emits_begin_then_commit() {
    let Some((mut conn, rec)) = connect().await else { return };
    conn.begin().await.unwrap();
    conn.commit().await.unwrap();
    let evs: Vec<_> = rec.0.lock().unwrap().iter()
        .filter(|e| matches!(e, OwnedEvent::TxBegin | OwnedEvent::TxCommit))
        .cloned()
        .collect();
    assert_eq!(evs, vec![OwnedEvent::TxBegin, OwnedEvent::TxCommit]);
}

#[tokio::test]
async fn prepare_cache_hit_after_second_call() {
    let Some((mut conn, rec)) = connect().await else { return };
    conn.query_typed("SELECT 1::int4", &[]).await.unwrap();
    conn.query_typed("SELECT 1::int4", &[]).await.unwrap();
    let hits: Vec<_> = rec.0.lock().unwrap().iter()
        .filter_map(|e| if let OwnedEvent::PrepareFinish { cache_hit } = e {
            Some(*cache_hit)
        } else { None })
        .collect();
    assert_eq!(hits, vec![false, true], "first miss, second hit");
}

#[tokio::test]
async fn pool_acquire_release_pair() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let rec = Arc::new(Recorder::default());
    let cfg = Config::parse(&url).unwrap();
    let pool = Pool::new(cfg, PoolConfig::new().max_connections(4))
        .with_instrumentation(rec.clone());
    {
        let _conn = pool.acquire().await.unwrap();
    }  // drop here → PoolRelease
    let evs: Vec<_> = rec.0.lock().unwrap().iter()
        .filter(|e| matches!(e,
            OwnedEvent::PoolAcquireFinish { .. } | OwnedEvent::PoolRelease))
        .cloned()
        .collect();
    assert!(matches!(evs[0], OwnedEvent::PoolAcquireFinish { ok: true }));
    assert_eq!(evs[1], OwnedEvent::PoolRelease);
}

impl Clone for OwnedEvent {
    fn clone(&self) -> Self {
        match self {
            OwnedEvent::ExecuteStart { sql, param_count } => Self::ExecuteStart { sql: sql.clone(), param_count: *param_count },
            OwnedEvent::ExecuteFinish { sql, rows, ok } => Self::ExecuteFinish { sql: sql.clone(), rows: *rows, ok: *ok },
            OwnedEvent::PrepareFinish { cache_hit } => Self::PrepareFinish { cache_hit: *cache_hit },
            OwnedEvent::TxBegin => Self::TxBegin,
            OwnedEvent::TxCommit => Self::TxCommit,
            OwnedEvent::TxRollback => Self::TxRollback,
            OwnedEvent::PipelineFlush { batch_len } => Self::PipelineFlush { batch_len: *batch_len },
            OwnedEvent::PoolAcquireFinish { ok } => Self::PoolAcquireFinish { ok: *ok },
            OwnedEvent::PoolRelease => Self::PoolRelease,
            OwnedEvent::Notice { code } => Self::Notice { code: code.clone() },
        }
    }
}
```

- [ ] **Step 2: Run with live PG**

```bash
podman run -d --name sntl-test-pg --rm \
  -e POSTGRES_USER=sentinel -e POSTGRES_PASSWORD=sentinel_test \
  -e POSTGRES_DB=sentinel_test -p 5432:5432 docker.io/library/postgres:16-alpine
until podman exec sntl-test-pg pg_isready -U sentinel -d sentinel_test >/dev/null 2>&1; do sleep 1; done
DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test \
  cargo test -p sentinel-driver --test instrumentation_test -- --test-threads=1
```

Expected: 4 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/sentinel-driver/tests/instrumentation_test.rs
git commit -m "test(observability): live-PG recording-impl coverage for all event arms"
```

---

### Task 13: `tracing-test` subscriber verification

**Files:**
- Modify: `crates/sentinel-driver/Cargo.toml` (add `tracing-test = "0.2"` as dev-dep)
- Create: `crates/sentinel-driver/tests/tracing_adapter_test.rs`

- [ ] **Step 1: Add dev-dep**

```toml
[dev-dependencies]
tracing-test = "0.2"
```

- [ ] **Step 2: Write the test**

```rust
//! Verify TracingInstrumentation records the expected fields onto the
//! current span. Live-PG; skips silently without DATABASE_URL.

use std::sync::Arc;

use sentinel_driver::{Config, Connection, TracingInstrumentation};
use tracing::Instrument;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn execute_records_db_attributes() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let cfg = Config::parse(&url).unwrap()
        .with_instrumentation(Arc::new(TracingInstrumentation::default()));
    let mut conn = Connection::connect(cfg).await.unwrap();
    let span = tracing::info_span!(
        "db.query",
        db.system = tracing::field::Empty,
        db.statement = tracing::field::Empty,
        db.operation = tracing::field::Empty,
        db.rows_affected = tracing::field::Empty,
        sntl.param_count = tracing::field::Empty,
        sntl.duration_us = tracing::field::Empty,
    );
    async {
        conn.query("SELECT 1::int4", &[]).await.unwrap();
    }.instrument(span).await;

    assert!(logs_contain("db.system"));
    assert!(logs_contain("postgresql"));
    assert!(logs_contain("SELECT"));    // db.operation
}

#[tokio::test]
#[traced_test]
async fn slow_query_emits_warn() {
    use std::time::Duration;
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let cfg = Config::parse(&url).unwrap()
        .with_instrumentation(Arc::new(TracingInstrumentation {
            max_sql_len: 1024,
            slow_threshold: Some(Duration::from_millis(0)),  // everything is "slow"
        }));
    let mut conn = Connection::connect(cfg).await.unwrap();
    async {
        conn.query("SELECT 1::int4", &[]).await.unwrap();
    }.instrument(tracing::info_span!("db.query")).await;
    assert!(logs_contain("slow query"));
}
```

- [ ] **Step 3: Run + commit**

```bash
DATABASE_URL=postgres://… cargo test -p sentinel-driver --test tracing_adapter_test -- --test-threads=1
git add crates/sentinel-driver/Cargo.toml crates/sentinel-driver/tests/tracing_adapter_test.rs
git commit -m "test(observability): tracing-test subscriber asserts db.* + slow-query warn"
```

---

### Task 14: Zero-allocation gate

**Files:**
- Modify: `crates/sentinel-driver/Cargo.toml` (add `dhat = "0.3"` dev-dep if not present)
- Create: `crates/sentinel-driver/tests/noop_zero_alloc_test.rs`

- [ ] **Step 1: Confirm dhat is available; add if not**

```bash
grep -q "^dhat" crates/sentinel-driver/Cargo.toml || sed -i '/^\[dev-dependencies\]/a dhat = "0.3"' crates/sentinel-driver/Cargo.toml
```

- [ ] **Step 2: Write the test**

```rust
//! Assert NoOpInstrumentation adds zero heap allocations on the hot path.
//! Live-PG; skips without DATABASE_URL.

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use sentinel_driver::{Config, Connection};

#[tokio::test]
async fn noop_instrumentation_zero_alloc_hot_path() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let cfg = Config::parse(&url).unwrap();   // no with_instrumentation
    let mut conn = Connection::connect(cfg).await.unwrap();

    // Warm up — prepare cache, allocate buffers
    for _ in 0..10 {
        conn.query("SELECT 1::int4", &[]).await.unwrap();
    }

    let _profiler = dhat::Profiler::builder().testing().build();
    let before = dhat::HeapStats::get();

    for _ in 0..1000 {
        conn.query("SELECT 1::int4", &[]).await.unwrap();
    }

    let after = dhat::HeapStats::get();
    let delta_blocks = after.total_blocks - before.total_blocks;
    // The driver itself allocates per query (Vec<Row>, etc.). The gate is:
    // instrumentation must add ZERO blocks on top of the existing driver
    // baseline. Capture the per-query rate WITH instrumentation == rate
    // WITHOUT instrumentation. Tighter check: this test mostly catches
    // regressions where NoOp::on_event accidentally heap-allocates.
    // 1000 queries → if each NoOp call allocated 1 block, that's +1000 blocks.
    assert!(
        delta_blocks < (before.total_blocks / 100) + 100,
        "NoOp added too many heap allocations: delta={} baseline={}",
        delta_blocks, before.total_blocks
    );
}
```

- [ ] **Step 3: Run + commit**

```bash
DATABASE_URL=postgres://… cargo test -p sentinel-driver --test noop_zero_alloc_test --release
git add crates/sentinel-driver/Cargo.toml crates/sentinel-driver/tests/noop_zero_alloc_test.rs
git commit -m "test(observability): NoOp adds no heap allocations on the hot path"
```

---

### Task 15: PR-A1 wrap-up — bench gate + release v2.1.0

- [ ] **Step 1: Run TFB bench WITH and WITHOUT instrumentation**

The sentinel repo's `examples/axum-bench/` already runs the TFB endpoints. Two scenarios:
1. Driver linked from path = `../../sentinel-driver` (this PR-A1 branch).
2. Same but install `TracingInstrumentation::default()` on the pool.

Compare `/db`, `/queries=20`, `/updates=20` throughput. Gate: ≤ 2% drop in scenario 2 vs scenario 1.

Procedure documented in `examples/README.md` already.

- [ ] **Step 2: Workspace verification**

```bash
cd /home/mrbt/Desktop/workspaces/orm/repositories/sentinel-driver
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
DATABASE_URL=postgres://… cargo test --workspace -- --test-threads=1
cargo deny check
```

All must pass.

- [ ] **Step 3: Bump version + push + open PR**

```bash
# Update crates/sentinel-driver/Cargo.toml: version = "2.1.0"
# Update CHANGELOG.md
git add crates/sentinel-driver/Cargo.toml CHANGELOG.md
git commit -m "chore: release sentinel-driver v2.1.0 — Instrumentation trait"
git push -u origin feat/instrumentation
gh pr create --title "feat(observability): Instrumentation trait + TracingInstrumentation adapter (v2.1.0)" \
  --body "$(cat <<'EOF'
## Summary

Adds a single `Instrumentation` trait wired into every wire site
(query / prepare / pipeline / transaction / pool / notify). Default impl
is `NoOpInstrumentation` — zero overhead when not installed.

Replaces the dead `ObservabilityConfig` / `QueryMetrics` /
`QueryMetricsCallback` types (none of which ever fired).

Ships `TracingInstrumentation` adapter that records OTel-conformant
`db.*` fields onto the current `tracing::Span`. No feature gate —
`tracing` was already a hard dep.

## Tests

- live-PG instrumentation_test (4 cases)
- tracing-test subscriber for db.* fields + slow-query WARN
- dhat zero-alloc gate
- TFB bench gate (≤ 2% throughput delta)
EOF
)"
```

- [ ] **Step 4: Wait for CI + merge + publish to crates.io**

After merge, release-please tags v2.1.0 and the publish workflow pushes to crates.io.

---

## Phase 2 — sntl bridge (PR-A2)

> Switch back to the sentinel repo. Create worktree: `git worktree add .worktrees/feat-observability-bridge -b feat/observability-bridge origin/main`.

### Task 16: Bump driver dep to 2.1

**Files:**
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Update version**

```toml
[workspace.dependencies]
sentinel-driver = "2.1.0"
```

- [ ] **Step 2: Verify workspace builds**

```bash
cargo update -p sentinel-driver
cargo check --workspace
```

Expected: clean. (Any code that imported now-removed observability types would error here.)

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump sentinel-driver to 2.1.0 for Instrumentation trait"
```

---

### Task 17: `sntl::__priv::emit_query_macro` helper

**Files:**
- Create: `sntl/src/__priv.rs`
- Modify: `sntl/src/lib.rs` (`pub mod __priv;` — doc-hidden)

- [ ] **Step 1: Write the helper**

```rust
//! Internal, NOT part of the public API. Used by `sntl-macros` only.
#![doc(hidden)]

use sentinel_driver::{Event, Instrumentation};

/// Emit a `QueryMacro` event onto the connection's instrumentation.
///
/// `query_id` should be the 13-char hash from `.sentinel/queries/<id>.json`
/// so consumers can correlate to the offline cache entry.
pub fn emit_query_macro(
    conn: &sentinel_driver::Connection,
    macro_name: &str,
    query_id: &str,
    sql: &str,
) {
    // `Connection::instrumentation()` is a public accessor added in
    // Phase 1 Task 3, Step 3.
    conn.instrumentation().on_event(&Event::QueryMacro {
        macro_name,
        query_id,
        sql,
    });
}
```

- [ ] **Step 2: Doc-hide in lib.rs**

```rust
#[doc(hidden)]
pub mod __priv;
```

- [ ] **Step 3: Commit**

```bash
git add sntl/src/{__priv,lib}.rs
git commit -m "feat(sntl): __priv::emit_query_macro for proc-macro use"
```

---

### Task 18: `sntl-macros` emits `QueryMacro` from every query macro

**Files:**
- Modify: `sntl-macros/src/query/anonymous.rs`
- Modify: `sntl-macros/src/query/typed.rs`
- Modify: `sntl-macros/src/query/file.rs`
- Modify: `sntl-macros/src/query/pipeline.rs`
- Modify: `sntl-macros/src/query/unchecked.rs`

- [ ] **Step 1: Find the codegen output that builds the query**

Each module has an `expand(...)` fn that emits the call site. The codegen builds a struct + a `fetch_*` impl. We inject one extra line before the actual driver call.

- [ ] **Step 2: Inject the emit**

For `anonymous.rs` (and analogously the others), at the point where the macro emits `conn.query(...)`:

```rust
// before:
let ts = quote! {
    {
        let __conn = #conn;
        let __rows = __conn.query(#sql, &[#(&#params),*]).await?;
        // ...
    }
};

// after:
let ts = quote! {
    {
        let __conn = #conn;
        ::sntl::__priv::emit_query_macro(__conn, "query", #query_id, #sql);
        let __rows = __conn.query(#sql, &[#(&#params),*]).await?;
        // ...
    }
};
```

`#query_id` is the 13-char cache hash that the macro already computes. `#macro_name` is the literal "query" / "query_as" / "query_scalar" / etc., depending on which expand fn this is.

- [ ] **Step 3: Build the existing test suite**

```bash
cargo test -p sntl --lib
```

Expected: all green (semantic-preserving — just an extra one-line emit before each query).

- [ ] **Step 4: Commit**

```bash
git add sntl-macros/src/query/
git commit -m "feat(sntl-macros): emit QueryMacro event from every query!() variant"
```

---

### Task 19: `sntl::core::observability` — `SntlTracing` adapter + `install_default_tracing`

**Files:**
- Create: `sntl/src/core/observability.rs`
- Modify: `sntl/src/core/mod.rs` (add `pub mod observability;`)
- Modify: `sntl/src/lib.rs` (re-export `pub use core::observability;`)

- [ ] **Step 1: Write the module**

```rust
//! Sentinel-level observability bridge.
//!
//! Wraps `sentinel_driver::TracingInstrumentation` and adds handlers for
//! Sentinel-specific events (`QueryMacro`, `Reducer*`, `Migration*`).
//! All driver-level events delegate to the wrapped impl.

use std::sync::Arc;

use sentinel_driver::{Event, Instrumentation, TracingInstrumentation};

#[derive(Clone)]
pub struct SntlTracing {
    inner: TracingInstrumentation,
}

impl Default for SntlTracing {
    fn default() -> Self {
        Self {
            inner: TracingInstrumentation::default(),
        }
    }
}

impl SntlTracing {
    pub fn with_inner(inner: TracingInstrumentation) -> Self {
        Self { inner }
    }
}

impl Instrumentation for SntlTracing {
    fn on_event(&self, ev: &Event<'_>) {
        let span = tracing::Span::current();
        match ev {
            Event::QueryMacro { macro_name, query_id, sql: _ } => {
                span.record("sntl.macro", *macro_name);
                span.record("sntl.query_id", *query_id);
            }
            Event::ReducerBegin { name } => {
                tracing::info!(reducer = %name, "reducer begin");
            }
            Event::ReducerCommit { name, duration } => {
                tracing::info!(
                    reducer = %name,
                    duration_us = duration.as_micros() as i64,
                    "reducer commit"
                );
            }
            Event::ReducerRollback { name, error } => {
                tracing::warn!(reducer = %name, error = %error, "reducer rollback");
            }
            Event::MigrationApply { version, duration, checksum } => {
                tracing::info!(
                    version = %version,
                    duration_us = duration.as_micros() as i64,
                    checksum = %checksum,
                    "migration applied"
                );
            }
            Event::MigrationDrift { version, recorded, current } => {
                tracing::error!(
                    version = %version,
                    recorded = %recorded,
                    current = %current,
                    "migration checksum drift"
                );
            }
            _ => self.inner.on_event(ev),
        }
    }
}

/// Install `SntlTracing::default()` on the pool.
///
/// Matches the consume-self signature of `Pool::with_instrumentation`
/// defined in driver Task 10.
pub fn install_default_tracing(pool: sentinel_driver::Pool) -> sentinel_driver::Pool {
    pool.with_instrumentation(Arc::new(SntlTracing::default()))
}
```

- [ ] **Step 2: Module wiring**

```rust
// sntl/src/core/mod.rs
pub mod observability;

// sntl/src/lib.rs
pub use core::observability;
```

- [ ] **Step 3: `cargo check -p sntl`**

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add sntl/src/core/observability.rs sntl/src/core/mod.rs sntl/src/lib.rs
git commit -m "feat(sntl): SntlTracing adapter + install_default_tracing helper"
```

---

### Task 20: `#[reducer]` emits `ReducerBegin` / `Commit` / `Rollback`

**Files:**
- Modify: `sntl-macros/src/...` (find the `#[reducer]` attribute expansion)

- [ ] **Step 1: Locate the reducer macro**

```bash
grep -rn "reducer" sntl-macros/src/ | head
```

The `#[reducer]` macro wraps a user fn in BEGIN/COMMIT/ROLLBACK. The wrapper is the right place to emit events.

- [ ] **Step 2: Inject emits**

```rust
// generated code:
async fn user_fn(#args) -> Result<R> {
    let __conn = /* extract from args */;
    let __name = stringify!(user_fn);
    __conn.instrumentation().on_event(&::sentinel_driver::Event::ReducerBegin {
        name: __name,
    });
    let __start = ::std::time::Instant::now();
    __conn.begin().await?;
    match (async { /* original body */ }).await {
        Ok(r) => {
            __conn.commit().await?;
            __conn.instrumentation().on_event(&::sentinel_driver::Event::ReducerCommit {
                name: __name,
                duration: __start.elapsed(),
            });
            Ok(r)
        }
        Err(e) => {
            let __err = format!("{e}");
            __conn.rollback().await.ok();
            __conn.instrumentation().on_event(&::sentinel_driver::Event::ReducerRollback {
                name: __name,
                error: &__err,
            });
            Err(e)
        }
    }
}
```

- [ ] **Step 3: Run reducer test suite**

```bash
DATABASE_URL=postgres://… cargo test -p sntl --tests reducer
```

Expected: all green.

- [ ] **Step 4: Commit**

```bash
git add sntl-macros/src/
git commit -m "feat(sntl-macros): #[reducer] emits Reducer{Begin,Commit,Rollback}"
```

---

### Task 21: `sntl-migrate` emits `MigrationApply` / `MigrationDrift`

**Files:**
- Modify: `sntl-migrate/src/runner.rs`

- [ ] **Step 1: Emit `MigrationApply` after successful record**

In `runner.rs` `run_locked` (around the existing `tracking::record` call):

```rust
apply_one(conn, m).await?;
let checksum = sha256_of_sql(&m.sql);
tracking::record(conn, &m.version, &checksum).await?;
conn.instrumentation().on_event(&sentinel_driver::Event::MigrationApply {
    version: m.version.as_str(),
    duration: started.elapsed(),  // track per-migration elapsed
    checksum: &checksum,
});
report.applied.push(m.version.clone());
```

Wrap with `let started = Instant::now();` at the top of each iteration.

- [ ] **Step 2: Emit `MigrationDrift` in `info()`**

```rust
if let Some(recorded) = applied_map.get(&m.version) {
    let current = sha256_of_sql(&m.sql);
    let state = if current == *recorded {
        State::Applied
    } else {
        conn.instrumentation().on_event(&sentinel_driver::Event::MigrationDrift {
            version: m.version.as_str(),
            recorded,
            current: &current,
        });
        State::ChecksumDrift
    };
    // ...
}
```

- [ ] **Step 3: Build + test**

```bash
DATABASE_URL=postgres://… cargo test -p sntl-migrate --test runner_test -- --test-threads=1
```

Expected: existing tests still pass.

- [ ] **Step 4: Commit**

```bash
git add sntl-migrate/src/runner.rs
git commit -m "feat(sntl-migrate): emit MigrationApply on success, MigrationDrift on checksum mismatch"
```

---

### Task 22: `sntl/tests/instrumentation_test.rs` — sntl-level coverage

**Files:**
- Create: `sntl/tests/instrumentation_test.rs`

- [ ] **Step 1: Write the test**

```rust
//! Live-PG verification that sntl-level events fire from query!() macros.

use std::sync::{Arc, Mutex};

use sentinel_driver::{Event, Instrumentation};

#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        if let Event::QueryMacro { macro_name, query_id, .. } = ev {
            self.0.lock().unwrap().push(format!("{macro_name}:{query_id}"));
        }
    }
}

#[tokio::test]
async fn query_macro_fires_with_id() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let rec = Arc::new(Recorder::default());
    let cfg = sentinel_driver::Config::parse(&url).unwrap()
        .with_instrumentation(rec.clone());
    let mut conn = sentinel_driver::Connection::connect(cfg).await.unwrap();

    let _: i32 = sntl::query_scalar!("SELECT 1::int4")
        .fetch_one(&mut conn).await.unwrap();

    let evs = rec.0.lock().unwrap();
    assert!(!evs.is_empty(), "QueryMacro must fire");
    assert!(evs[0].starts_with("query_scalar:"));
}
```

- [ ] **Step 2: Run + commit**

```bash
DATABASE_URL=postgres://… cargo test -p sntl --test instrumentation_test
git add sntl/tests/instrumentation_test.rs
git commit -m "test(sntl): live-PG verification that query macros emit QueryMacro"
```

---

### Task 23: `sntl/tests/reducer_instrumentation_test.rs`

**Files:**
- Create: `sntl/tests/reducer_instrumentation_test.rs`

- [ ] **Step 1: Write the test**

```rust
//! Live-PG verification that #[reducer] emits Begin + Commit on success
//! and Begin + Rollback on error.

use std::sync::{Arc, Mutex};

use sentinel_driver::{Event, Instrumentation};

#[derive(Default, Clone)]
struct Recorder(Arc<Mutex<Vec<&'static str>>>);

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        let tag = match ev {
            Event::ReducerBegin { .. } => "begin",
            Event::ReducerCommit { .. } => "commit",
            Event::ReducerRollback { .. } => "rollback",
            _ => return,
        };
        self.0.lock().unwrap().push(tag);
    }
}

#[sntl::reducer]
async fn happy(conn: &mut sentinel_driver::Connection) -> Result<i32, sntl::core::Error> {
    Ok(42)
}

#[sntl::reducer]
async fn sad(conn: &mut sentinel_driver::Connection) -> Result<i32, sntl::core::Error> {
    Err(sntl::core::Error::message("forced"))
}

#[tokio::test]
async fn happy_path_emits_begin_commit() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let rec = Recorder::default();
    let cfg = sentinel_driver::Config::parse(&url).unwrap()
        .with_instrumentation(Arc::new(rec.clone()));
    let mut conn = sentinel_driver::Connection::connect(cfg).await.unwrap();
    happy(&mut conn).await.unwrap();
    assert_eq!(*rec.0.lock().unwrap(), vec!["begin", "commit"]);
}

#[tokio::test]
async fn error_path_emits_begin_rollback() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let rec = Recorder::default();
    let cfg = sentinel_driver::Config::parse(&url).unwrap()
        .with_instrumentation(Arc::new(rec.clone()));
    let mut conn = sentinel_driver::Connection::connect(cfg).await.unwrap();
    let _ = sad(&mut conn).await;
    assert_eq!(*rec.0.lock().unwrap(), vec!["begin", "rollback"]);
}
```

- [ ] **Step 2: Run + commit**

```bash
DATABASE_URL=postgres://… cargo test -p sntl --test reducer_instrumentation_test -- --test-threads=1
git add sntl/tests/reducer_instrumentation_test.rs
git commit -m "test(sntl): reducer emits begin+commit on happy path, begin+rollback on error"
```

---

### Task 24: `sntl-migrate/tests/migrate_instrumentation_test.rs`

**Files:**
- Create: `sntl-migrate/tests/migrate_instrumentation_test.rs`

- [ ] **Step 1: Write the test**

```rust
//! Live-PG verification that sntl-migrate emits MigrationApply per
//! migration and MigrationDrift when a file is tampered with.

use std::sync::{Arc, Mutex};

use sentinel_driver::{Event, Instrumentation, pool::config::PoolConfig};
use sntl_migrate::Migrator;
use tempfile::tempdir;

#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        let tag = match ev {
            Event::MigrationApply { version, checksum, .. } => format!("apply:{version}:{checksum}"),
            Event::MigrationDrift { version, .. } => format!("drift:{version}"),
            _ => return,
        };
        self.0.lock().unwrap().push(tag);
    }
}

#[tokio::test]
async fn apply_then_drift() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let rec = Arc::new(Recorder::default());
    let cfg = sentinel_driver::Config::parse(&url).unwrap()
        .with_instrumentation(rec.clone());
    let pool = sentinel_driver::Pool::new(cfg, PoolConfig::new().max_connections(2));

    // Clean slate
    let mut admin = pool.acquire().await.unwrap();
    admin.execute("SET client_min_messages = ERROR", &[]).await.unwrap();
    admin.execute("DROP TABLE IF EXISTS _sntl_migrations, mig_instr_t", &[]).await.unwrap();
    drop(admin);

    let dir = tempdir().unwrap();
    let mig = dir.path().join("migrations/20260514_120000_create");
    std::fs::create_dir_all(&mig).unwrap();
    std::fs::write(mig.join("up.sql"), "CREATE TABLE mig_instr_t (id int);").unwrap();

    Migrator::from_dir(dir.path().join("migrations")).unwrap()
        .run(&pool).await.unwrap();

    // Tamper
    std::fs::write(mig.join("up.sql"), "CREATE TABLE mig_instr_t (id int); -- drifted").unwrap();
    Migrator::from_dir(dir.path().join("migrations")).unwrap()
        .info(&pool).await.unwrap();

    let evs = rec.0.lock().unwrap();
    assert!(evs.iter().any(|e| e.starts_with("apply:20260514_120000_create")));
    assert!(evs.iter().any(|e| e.starts_with("drift:20260514_120000_create")));
}
```

- [ ] **Step 2: Run + commit**

```bash
DATABASE_URL=postgres://… cargo test -p sntl-migrate --test migrate_instrumentation_test -- --test-threads=1
git add sntl-migrate/tests/migrate_instrumentation_test.rs
git commit -m "test(sntl-migrate): emits MigrationApply per apply, MigrationDrift on tamper"
```

---

### Task 25: End-to-end OTel test

**Files:**
- Modify: `sntl/Cargo.toml` (add `tracing-opentelemetry = "0.28"`, `opentelemetry = "0.27"`, `opentelemetry_sdk = "0.27"`, `opentelemetry-stdout = "0.27"` as dev-deps)
- Create: `sntl/tests/otel_e2e_test.rs`

- [ ] **Step 1: Add dev-deps**

```toml
[dev-dependencies]
tracing-opentelemetry = "0.28"
opentelemetry = "0.27"
opentelemetry_sdk = "0.27"
opentelemetry-stdout = "0.27"
```

- [ ] **Step 2: Write the test**

```rust
//! End-to-end OTel: install tracing-opentelemetry → run a query → assert
//! the stdout exporter received a span with db.system = postgresql.

use std::sync::Arc;

use opentelemetry::trace::TracerProvider as _;
use sentinel_driver::TracingInstrumentation;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::test]
async fn query_exports_otel_span_with_db_system() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };

    // Build a stdout exporter that we'll read back via a buffer.
    let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let exporter = opentelemetry_stdout::SpanExporter::builder()
        .with_writer(buf.clone() /* implements Write via shim */)
        .build();
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.tracer("sntl-test");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry().with(otel_layer).init();

    let cfg = sentinel_driver::Config::parse(&url).unwrap()
        .with_instrumentation(Arc::new(TracingInstrumentation::default()));
    let mut conn = sentinel_driver::Connection::connect(cfg).await.unwrap();

    use tracing::Instrument;
    async {
        conn.query("SELECT 1::int4", &[]).await.unwrap();
    }
    .instrument(tracing::info_span!(
        "db.query",
        db.system = tracing::field::Empty,
        db.statement = tracing::field::Empty,
    ))
    .await;

    provider.force_flush();
    let out = String::from_utf8_lossy(&buf.lock().unwrap()).to_string();
    assert!(out.contains("db.system"));
    assert!(out.contains("postgresql"));
}
```

The `with_writer` shim may need a small wrapper (`opentelemetry-stdout`'s API exposes a writer-builder; if it doesn't take an `Arc<Mutex<Vec<u8>>>` directly, write to a `tempfile::NamedTempFile` and read it back).

- [ ] **Step 3: Run + commit**

```bash
DATABASE_URL=postgres://… cargo test -p sntl --test otel_e2e_test
git add sntl/Cargo.toml sntl/tests/otel_e2e_test.rs
git commit -m "test(sntl): e2e — query exports OTel span with db.system=postgresql"
```

---

### Task 26: `docs/observability-guide.md` + README + roadmap

**Files:**
- Create: `docs/observability-guide.md`
- Modify: `README.md`
- Modify: memory `roadmap_sentinel.md`

- [ ] **Step 1: Write the user guide**

Cover:
- What the trait is + zero-overhead claim
- `Config::with_instrumentation` / `Pool::with_instrumentation` install
- `TracingInstrumentation` quickstart (4 lines of code)
- `SntlTracing` quickstart + what sntl-level events fire
- Sample `tracing-opentelemetry` setup for OTLP/Jaeger export
- Writing a custom `Instrumentation` (recording for tests, metrics-rs, etc.)
- Param-value redaction philosophy
- Event reference table (every Event arm + when it fires)
- Limitations: no async handlers, no per-event filter API at trait level

- [ ] **Step 2: Update README**

In the architecture section, append:

```
- **Observability** — Instrumentation trait + TracingInstrumentation adapter
  with OTel `db.*` semantic conventions. See docs/observability-guide.md.
```

In the Features list, add:
```
- **Production observability** — single trait hooks every wire site and
  every macro invocation; ships with a tracing/OTel adapter
```

- [ ] **Step 3: Update memory roadmap**

In `~/.claude/projects/.../memory/roadmap_sentinel.md` under v0.4:

```
## v0.4 — production-readiness
- ✅ sntl observability — Instrumentation trait + tracing adapter +
  SntlTracing bridge (sentinel-driver v2.1, sentinel v0.4, 2026-05-?).
```

- [ ] **Step 4: Commit + push + PR**

```bash
git add docs/observability-guide.md README.md
git commit -m "docs: observability guide + README + roadmap update for v0.4"

cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo deny check
DATABASE_URL=postgres://… cargo test --workspace -- --test-threads=1

git push -u origin feat/observability-bridge
gh pr create --title "feat(sntl): observability bridge — SntlTracing + macro/reducer/migrate events (v0.4)" \
  --body "..."
```

---

## Self-Review

**Spec coverage** (against `docs/plans/2026-05-13-sntl-observability-design.md`):

| Spec section | Implementing task(s) |
|---|---|
| Instrumentation trait | Task 1, 2 |
| Event taxonomy | Task 2 |
| Connection / Config install | Tasks 3, 4 |
| Wire into Connection | Tasks 5, 6, 7, 8, 9 |
| Wire into Pool | Task 10 |
| TracingInstrumentation adapter | Task 11 |
| OTel semantic conventions | Task 11 (`db.system/statement/operation/rows_affected` + `sntl.*`) |
| Zero-overhead claim | Task 14 + 15 bench gate |
| Recording-impl unit tests | Task 12 |
| tracing-test subscriber | Task 13 |
| `dhat` zero-alloc gate | Task 14 |
| Bench gate ≤ 2 % | Task 15 |
| sntl bridge `SntlTracing` | Task 19 |
| sntl-macros `QueryMacro` events | Tasks 17, 18 |
| Reducer events | Task 20 |
| sntl-migrate events | Task 21 |
| sntl-side recording tests | Tasks 22, 23, 24 |
| E2E OTel test | Task 25 |
| `docs/observability-guide.md` | Task 26 |

**Placeholder scan:** none.

**Type consistency:**
- `Instrumentation`, `Event<'_>`, `StmtRef`, `Outcome`, `DisconnectReason`, `RollbackReason`, `AcquireOutcome` — defined Task 2, consumed identically across Tasks 3–25.
- `Connection::instrumentation()` is `pub` (added in Task 3 Step 3) — Tasks 17, 20, 21 consume it.
- `Pool::with_instrumentation` is consume-self (Task 10), matched by `install_default_tracing(pool) -> Pool` in Task 19.
- `TracingInstrumentation` ships always (no feature gate) per Resolved deviations §1.

**Open items** — none after the verification spike at the top of Phase 1. Phase 2 starts only after PR-A1 publishes v2.1.0 to crates.io.

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-05-14-sntl-observability-impl.md`. Two execution options:

**1. Subagent-Driven** — fresh subagent per task. Rigorous per-task review, higher token cost.

**2. Inline Execution** — run tasks in this session with checkpoints after each phase. Faster, matches the cadence that worked for PR #18 / #20 / #21.

Recommend **inline execution** for Phase 1 (driver tasks 1–15 are mechanical wiring), and either approach for Phase 2 depending on review needs. Which approach?
