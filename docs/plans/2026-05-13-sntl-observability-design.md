# `sntl-observability` — Design (v0.4)

## Goal

Give Sentinel a first-class observability surface so production users can wire
in tracing / OpenTelemetry / custom metrics without touching the hot path.
Close the largest remaining production-readiness gap vs sqlx / Diesel 2.2.

Today the driver ships `ObservabilityConfig` and `QueryMetricsCallback`, but
neither is wired — they're dead types. Replacing them with a working trait is
strictly additive for current consumers (nothing they depend on fires today).

## Non-goals (v0.4)

- `metrics-rs` adapter — capturable from the trait, no need to ship it.
- Prometheus / Jaeger / OTLP exporter — covered by `tracing-opentelemetry`
  downstream.
- Async event handlers — `on_event` is sync; users can enqueue inside.
- Per-event filter API — adapters do their own filtering.
- Parameter-value logging — security risk; never default; users write their
  own adapter if they need it.
- Slow-query log file — `tracing::warn!` emission covers it.

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│ user app (axum, etc.)                                          │
│  └─ tracing-subscriber + tracing-opentelemetry                 │
│     (consumer-installed, not shipped here)                     │
└────────────────────────────────────────────────────────────────┘
                          ▲ subscribe
┌────────────────────────────────────────────────────────────────┐
│ sentinel (PR-A2)                                               │
│  • sntl::observability::SntlTracing (extends driver adapter)   │
│  • sntl-macros: query!() emits QueryMacro events               │
│  • Reducer Begin/Commit/Rollback                               │
│  • sntl-migrate: MigrationApply / MigrationDrift               │
└────────────────────────────────────────────────────────────────┘
                          │ uses
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ sentinel-driver v2.1.0 (PR-A1)                                 │
│  pub trait Instrumentation: Send + Sync + 'static {            │
│      fn on_event(&self, event: &Event<'_>);                    │
│  }                                                             │
│  • Wired into Connection (Prepare/Execute/Notify/Disconnect)   │
│  • Wired into PipelineBatch (Start/Flush)                      │
│  • Wired into Pool (Acquire/Release)                           │
│  • Wired into Transaction (Begin/Commit/Rollback)              │
│                                                                │
│  feature `tracing` → ships TracingInstrumentation adapter      │
│  (OTel-conformant db.* span fields).                           │
└────────────────────────────────────────────────────────────────┘
```

**Install model:** `Arc<dyn Instrumentation>` held by `Connection`, set via
`Config::with_instrumentation`, `Pool::with_instrumentation`, or
`Connection::set_instrumentation`. NoOp default means zero overhead when not
installed.

**Removed:** `ObservabilityConfig`, `QueryMetrics`, `QueryMetricsCallback`,
`log_slow_query`, `query_span` — all dead code today, replaced cleanly.

## Event taxonomy

```rust
#[non_exhaustive]
pub enum Event<'a> {
    // Connection lifecycle
    Connect       { host: &'a str, port: u16 },
    Authenticated { user: &'a str, params: &'a [(String, String)] },
    Disconnect    { reason: DisconnectReason },

    // Query lifecycle
    PrepareStart  { name: &'a str, sql: &'a str },
    PrepareFinish { name: &'a str, param_oids: &'a [u32], col_count: u16,
                    duration: Duration, cache_hit: bool },
    ExecuteStart  { stmt: StmtRef<'a>, param_count: usize },
    ExecuteFinish { stmt: StmtRef<'a>, rows: u64, duration: Duration,
                    outcome: Outcome<'a> },

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

    // PG asynchronous messages
    Notice       { severity: &'a str, code: &'a str, message: &'a str },
    Notification { channel: &'a str, payload: &'a str, pid: i32 },

    // Sentinel-level (emitted only by sntl bridge, never by driver itself)
    QueryMacro      { macro_name: &'a str, query_id: &'a str, sql: &'a str },
    ReducerBegin    { name: &'a str },
    ReducerCommit   { name: &'a str, duration: Duration },
    ReducerRollback { name: &'a str, error: &'a str },
    MigrationApply  { version: &'a str, duration: Duration, checksum: &'a str },
    MigrationDrift  { version: &'a str, recorded: &'a str, current: &'a str },
}

pub enum StmtRef<'a> {
    Named  { name: &'a str },
    Inline { sql: &'a str },
}

pub enum Outcome<'a> { Ok, Err(&'a Error) }
pub enum DisconnectReason { Graceful, BrokenPipe, Timeout, ServerKill }
pub enum RollbackReason<'a> { Explicit, Drop, Error(&'a Error) }
pub enum AcquireOutcome { Ok, Timeout, PoolClosed }
```

**Borrowing model:** events use `&'a str` so no allocation on the hot path.
Adapters clone inside their handler if they need to retain data.

**SQL truncation:** driver does NOT truncate. The `TracingInstrumentation`
adapter ships with `max_sql_len: usize` (default 1024).

**Param redaction:** driver never includes parameter values in events.
Default-safe.

**Span lifecycle:** `*Start` / `*Finish` events fire around the operation.
The trait stays event-only (no span object held). Adapters that want spans
pair them through `tracing`'s span machinery — `Span::enter()` for sync
sites and `Span::in_scope(|| ...)` / `Instrument::instrument(span)` for
async sites. No raw `thread_local!` storage; tracing-core already handles
async propagation correctly.

## API shape

### Driver-side trait

```rust
// sentinel-driver — src/instrumentation.rs
pub trait Instrumentation: Send + Sync + 'static {
    fn on_event(&self, event: &Event<'_>);
}

pub(crate) struct NoOpInstrumentation;
impl Instrumentation for NoOpInstrumentation {
    fn on_event(&self, _: &Event<'_>) {}
}
```

### Install paths

```rust
// At Config level — every connection / pool inherits.
let cfg = Config::parse(url)?
    .with_instrumentation(Arc::new(MyInstrumentation));

// Directly on Pool.
let pool = Pool::new(cfg, PoolConfig::new())
    .with_instrumentation(Arc::new(TracingInstrumentation::default()));

// Per-Connection (raw users).
let mut conn = Connection::connect(cfg).await?;
conn.set_instrumentation(Arc::new(MyInstrumentation));
```

### Sentinel bridge

```rust
// sntl::observability
pub fn install_default_tracing(pool: &mut sentinel_driver::Pool) {
    pool.set_instrumentation(Arc::new(
        sntl::observability::SntlTracing::default()
    ));
}
```

`SntlTracing` wraps `sentinel_driver::TracingInstrumentation`, matches sntl
event arms (`QueryMacro`, `Reducer*`, `Migration*`), and delegates the rest.

Macros emit events via:

```rust
// generated by sntl::query!() expansion
let _g = ::sntl::__priv::emit_query_macro("query", QUERY_ID, SQL, &conn);
// _g drops at scope end → ExecuteFinish paired by stack-local span guard
```

## Tracing adapter (driver, feature `tracing`)

```rust
#[cfg(feature = "tracing")]
pub struct TracingInstrumentation {
    pub max_sql_len: usize,             // default 1024
    pub slow_threshold: Option<Duration>,
}

impl Instrumentation for TracingInstrumentation {
    fn on_event(&self, ev: &Event<'_>) {
        match ev {
            Event::ExecuteStart { stmt, param_count } => {
                // Re-open the current span (provided by the caller via
                // `Span::enter` at the wire site) and decorate it.
                let span = tracing::Span::current();
                span.record("db.system", "postgresql");
                span.record("db.statement",  &%trunc(stmt.sql_or_name(),
                                                    self.max_sql_len));
                span.record("db.operation",  stmt.op_hint());
                span.record("sntl.param_count", param_count);
            }
            Event::ExecuteFinish { rows, duration, outcome, .. } => {
                let span = tracing::Span::current();
                span.record("db.rows_affected", rows);
                span.record("sntl.duration_us", duration.as_micros() as i64);
                if let Outcome::Err(e) = outcome {
                    span.record("error", true);
                    tracing::error!(error = %e, "query failed");
                }
                if matches!(self.slow_threshold, Some(t) if *duration > t) {
                    tracing::warn!(slow = true, "slow query");
                }
            }
            // TxBegin emits enclosing span; PipelineFlush emits batch span;
            // PoolAcquireFinish records wait; Notification opens span on
            // channel; etc.
            _ => {}
        }
    }
}
```

### OTel semantic conventions

- `db.system = "postgresql"`
- `db.statement` (truncated SQL)
- `db.operation` (parsed: SELECT / INSERT / UPDATE / DELETE / ...)
- `db.rows_affected`
- Custom Sentinel attrs: `sntl.duration_us`, `sntl.param_count`,
  `sntl.cache_hit`, `sntl.query_id`

Users running `tracing-opentelemetry` get OTLP / Jaeger / Zipkin export with
no extra glue.

## Zero-overhead claim

When no instrumentation is installed, every event site does:

```rust
self.instr.on_event(&Event::ExecuteStart { ... });
//   └─ vtable dispatch → NoOpInstrumentation::on_event → return
```

Bench target: **≤ 2 % throughput regression** on `examples/axum-bench/`
`/db` and `/queries=20` vs current driver. Gate the v2.1.0 release on this.

## Testing strategy

**Driver side (PR-A1)**

- `instrumentation_test.rs` — record events into `Mutex<Vec<OwnedEvent>>`,
  scripted ops, assert sequence + content.
- `tracing_adapter_test.rs` — `tracing-test` subscriber, assert spans emit
  the expected `db.*` fields.
- `noop_zero_alloc_test.rs` — `dhat` (existing dev-dep): 10k queries with
  NoOp, assert no allocations attributed to instrumentation sites.
- Live-PG smoke: install a recording instrumentation alongside the existing
  suite, assert ≥ 1 `ExecuteFinish` per real query.

**Sentinel side (PR-A2)**

- `sntl/tests/instrumentation_test.rs` — `query!()` / `query_as!()` /
  `query_pipeline!()` emit `QueryMacro` with the right `query_id`.
- `sntl/tests/reducer_instrumentation_test.rs` — succeed-and-panic reducers
  emit paired `ReducerBegin` + `ReducerCommit` / `ReducerRollback`.
- `sntl-migrate/tests/migrate_instrumentation_test.rs` — apply two
  migrations → `MigrationApply` per migration; tamper a file →
  `MigrationDrift`.
- End-to-end OTel: install `tracing-opentelemetry` with
  `opentelemetry-stdout`, run a query, capture JSON, assert `db.system =
  postgresql` plus child events.

## Release plan

| # | Repo | PR / version | Contents | Estimate |
|---|---|---|---|---|
| 1 | sentinel-driver | PR-A1 → **v2.1.0** | trait + Event + wire into Connection/PipelineBatch/Pool/Transaction + TracingInstrumentation under `tracing` feature + tests + bench gate | ~1 week |
| 2 | sentinel-driver | publish v2.1.0 | bump workspace, release-please tag | ~1 day |
| 3 | sentinel | PR-A2 | bump driver to 2.1; `sntl::observability`; sntl-macros emit `QueryMacro`; reducer events; sntl-migrate events; `install_default_tracing`; integration + e2e OTel tests; `docs/observability-guide.md` | ~1 week |
| 4 | sentinel | release v0.4.0 | bump version, update README + roadmap, publish | ~1 day |

## Risks + mitigations

| Risk | Mitigation |
|---|---|
| Driver perf regression ≥ 2 % | Bench gate in CI before merge; NoOp default; `#[inline]` on event-emit helpers |
| Span/event pairing breaks under async multi-task | Driver opens an enclosing `tracing::Span` per wire site (`span.in_scope` / `Instrument::instrument`) and lets tracing-core handle async propagation. The adapter only records fields onto the current span — no raw TLS. The trait itself stays event-only and async-safe. |
| sentinel-driver v2.1 breakage for current consumers | None expected — `ObservabilityConfig`/`QueryMetrics` are dead today; document in CHANGELOG |
| OTel semantic-convention churn | Document conventions used; pin to current; bump on next OTel major |

## Self-review checklist (for PR reviewers)

- [ ] Every wire site calling `on_event` passes a borrow, never owned data.
- [ ] Building with feature `tracing` on and off both pass `cargo check`.
- [ ] `cargo deny check` passes; `tracing-opentelemetry` lands only as a
      dev-dep.
- [ ] `examples/axum-bench/` shows ≤ 2 % delta.
- [ ] `docs/observability-guide.md` covers install + custom adapter + OTel.

## Open items resolved

1. **Adapter strategy** — trait + tracing adapter shipped in the driver
   under feature `tracing`. metrics-rs / Prometheus / OTel-native crates
   stay downstream.
2. **Backwards compatibility** — none needed; the removed callbacks never
   fire today. CHANGELOG entry only.
3. **Span lifecycle** — adapter, not trait, owns span pairing.
4. **Param redaction** — never emit; full stop.
5. **Migration of Sentinel-level events** — sntl bridge owns sntl-specific
   arms; driver never knows about them.
