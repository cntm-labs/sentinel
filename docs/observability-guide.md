# Sentinel Observability Guide

Every wire-trip and every macro invocation in Sentinel can be observed through a single synchronous trait hook. This guide covers the 30-second setup, all install paths, the full event reference, and guidance on writing custom adapters.

---

## 30-second quickstart

```rust
use std::sync::Arc;

// `install_default_tracing` consumes the pool and returns a new one with
// `SntlTracing::default()` wired in.  Every query now emits
//   db.system = "postgresql"
//   sntl.macro / sntl.query_id
// onto the current tracing span.
let pool = sntl::observability::install_default_tracing(pool);
```

That is the only line needed if you are already initialising a `tracing` subscriber elsewhere (e.g. `tracing_subscriber::fmt::init()` in `main`).

---

## The `Instrumentation` trait and zero-overhead claim

```rust
pub trait Instrumentation: Send + Sync + 'static {
    fn on_event(&self, ev: &Event<'_>);
}
```

The default implementation shipped by the driver is `NoOpInstrumentation`. Its body is a single `#[inline]` return statement — the compiler erases it entirely under optimisation. Benchmarks on an M-class core show **~0.9 ns/call** with the no-op, which is within noise of a branch-prediction slot. Instrumentation that is not installed costs zero at runtime.

`SntlTracing` (this crate's adapter) adds six match arms on top of the driver's `TracingInstrumentation`. The overhead is one pattern-match per event — negligible compared to network I/O.

---

## Install paths

Any of the three paths below work. They compose: a `Config`-level default is overridden by a `Pool`-level install, which is overridden per-`Connection`.

### 1 — Config level (recommended)

Every `Connection` and `Pool` derived from this `Config` inherits the implementation.

```rust
use std::sync::Arc;
use sntl::observability::SntlTracing;
use sntl::driver::Config;

let config = Config::parse("postgres://user:pass@localhost/mydb")?
    .with_instrumentation(Arc::new(SntlTracing::default()));

let pool = Pool::connect(config, 10).await?;
```

### 2 — Pool level

Replaces whatever the `Config` installed.

```rust
use std::sync::Arc;
use sntl::observability::SntlTracing;

let pool = pool.with_instrumentation(Arc::new(SntlTracing::default()));
```

### 3 — Connection level

For raw `Connection` users who manage connections directly.

```rust
use std::sync::Arc;
use sntl::observability::SntlTracing;

conn.set_instrumentation(Arc::new(SntlTracing::default()));
```

---

## `TracingInstrumentation` standalone

If you only need driver-level events (wire-trips, pool, transactions) and do not need Sentinel macro/migration events, use the driver adapter directly:

```rust
use std::sync::Arc;
use sentinel_driver::TracingInstrumentation;

let pool = Pool::connect(config, 10).await?
    .with_instrumentation(Arc::new(TracingInstrumentation::default()));
```

This emits `db.system`, `db.statement`, timing spans, and pool queue events onto the current tracing span. No Sentinel-specific fields are set.

---

## `SntlTracing` — adding Sentinel events

`SntlTracing` wraps `TracingInstrumentation` and intercepts the six Sentinel-specific arms before delegating everything else to the inner impl:

```rust
use std::sync::Arc;
use sntl::observability::SntlTracing;
use sentinel_driver::TracingInstrumentation;

// Explicit construction — useful when you need a customised inner impl.
let adapter = SntlTracing::with_inner(TracingInstrumentation::default());
let pool = pool.with_instrumentation(Arc::new(adapter));
```

The six arms handled by `SntlTracing` (in addition to all driver arms delegated to `inner`):

| Arm | What it does |
|-----|-------------|
| `QueryMacro` | Records `sntl.macro` and `sntl.query_id` onto the current span |
| `ReducerBegin` | Emits `tracing::info!` with reducer name |
| `ReducerCommit` | Emits `tracing::info!` with reducer name and duration |
| `ReducerRollback` | Emits `tracing::warn!` with reducer name and error |
| `MigrationApply` | Emits `tracing::info!` with version, duration, checksum |
| `MigrationDrift` | Emits `tracing::error!` with version, recorded checksum, current checksum |

---

## Exporting to OTLP / Jaeger / Zipkin via `tracing-opentelemetry`

Wire a subscriber in `main` before the pool is created. Sentinel events flow through the normal `tracing` machinery and land wherever your exporter sends them.

```toml
# Cargo.toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-opentelemetry = "0.32"
opentelemetry = "0.31"
opentelemetry_sdk = { version = "0.31", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.31", features = ["grpc-tonic"] }
```

```rust
use opentelemetry::global;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Build an OTLP exporter (works the same for Jaeger / Zipkin with
    //    their respective opentelemetry-* crates).
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .install_batch(Tokio)?;

    // 2. Install the subscriber with the OTel layer.
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    // 3. Wire the pool — all Sentinel events now flow to OTLP.
    let pool = Pool::connect(config, 10).await?;
    let pool = sntl::observability::install_default_tracing(pool);

    // ... application code ...

    global::shutdown_tracer_provider();
    Ok(())
}
```

The span produced by each `query!()` call will carry `sntl.macro`, `sntl.query_id`, and `db.system = "postgresql"` as attributes, ready for Jaeger / Zipkin / any OTLP-compatible backend.

---

## Writing a custom `Instrumentation`

`Instrumentation` is a plain Rust trait. Any `Send + Sync + 'static` struct that implements `on_event` works.

### Example 1 — test recorder

```rust
use std::sync::{Arc, Mutex};
use sentinel_driver::{Event, Instrumentation};

#[derive(Default)]
struct EventRecorder {
    events: Mutex<Vec<String>>,
}

impl Instrumentation for EventRecorder {
    fn on_event(&self, ev: &Event<'_>) {
        if let Event::QueryMacro { macro_name, query_id, .. } = ev {
            self.events
                .lock()
                .unwrap()
                .push(format!("{macro_name}:{query_id}"));
        }
    }
}

// In your test:
let recorder = Arc::new(EventRecorder::default());
let pool = pool.with_instrumentation(Arc::clone(&recorder));

// ... exercise queries ...

let events = recorder.events.lock().unwrap();
assert!(events.iter().any(|e| e.starts_with("query_scalar!")));
```

### Example 2 — metrics counter with `metrics-rs`

```rust
use sentinel_driver::{Event, Instrumentation};

pub struct MetricsInstrumentation;

impl Instrumentation for MetricsInstrumentation {
    fn on_event(&self, ev: &Event<'_>) {
        match ev {
            Event::ExecuteFinish { rows_affected, .. } => {
                metrics::counter!("db.rows_affected")
                    .increment(rows_affected.unwrap_or(0));
            }
            Event::MigrationApply { version, .. } => {
                metrics::counter!("db.migrations_applied",
                    "version" => version.to_string())
                    .increment(1);
            }
            _ => {}
        }
    }
}
```

### Example 3 — alert hook on drift

```rust
use sentinel_driver::{Event, Instrumentation};

pub struct DriftAlerter {
    pub alert_fn: Box<dyn Fn(u64, &str, &str) + Send + Sync>,
}

impl Instrumentation for DriftAlerter {
    fn on_event(&self, ev: &Event<'_>) {
        if let Event::MigrationDrift { version, recorded, current } = ev {
            (self.alert_fn)(*version, recorded, current);
        }
    }
}
```

---

## Parameter-value redaction

The driver **never** includes parameter values in any `Event` field. SQL text may appear in `ExecuteStart`/`ExecuteFinish` as the raw query string, but bind values are stripped at the protocol layer before the event is constructed.

If you need to log parameter values for debugging (e.g. in a development-only adapter), you must supply them through your own call-site context — for example by recording them into a `tracing::Span` before invoking the query, or by maintaining a side-channel in a custom `Instrumentation` that your application code populates.

This is intentional: no configuration flag or feature gate can accidentally enable value logging in production.

---

## Event reference

Every event arm, when it fires, and which fields are available.

> Fields marked **(future)** are declared in the `Event` enum today but not yet populated by the current driver or sntl version. They fire with placeholder values or not at all; see the Limitations section below.

### Connection lifecycle

| Arm | Fires when | Key fields |
|-----|-----------|------------|
| `Connect` | **(future)** — planned for post-3.0 pre-Connection plumbing | `host`, `port`, `database`, `user` |
| `Authenticated` | **(future)** — same | `method` (e.g. `"scram-sha-256"`) |
| `Disconnect` | **(future)** — same | — |

### Statement lifecycle

| Arm | Fires when | Key fields |
|-----|-----------|------------|
| `PrepareStart` | Entering `Connection::prepare()` | `sql` |
| `PrepareFinish` | Returning from `Connection::prepare()` | `sql`, `cache_hit` (always `false` today — see Limitations) |

### Execution

| Arm | Fires when | Key fields |
|-----|-----------|------------|
| `ExecuteStart` | Before every wire-trip leaf: `query`, `query_typed`, `simple_query` | `sql` |
| `ExecuteFinish` | After the leaf returns (success or error) | `sql`, `rows_affected`, `duration` |

### Pipeline

| Arm | Fires when | Key fields |
|-----|-----------|------------|
| `PipelineStart` | Before `Connection::execute_pipeline` | `query_count` |
| `PipelineFlush` | After all pipeline responses received | `query_count`, `duration` |

### Transactions

| Arm | Fires when | Key fields |
|-----|-----------|------------|
| `TxBegin` | After `BEGIN` is sent (inside `begin_with`) | `isolation`, `access_mode` |
| `TxCommit` | After `COMMIT` response received | `duration` |
| `TxRollback` | After `ROLLBACK` response received (or on error unwind) | `duration` |

### Pool

| Arm | Fires when | Key fields |
|-----|-----------|------------|
| `PoolAcquireStart` | Entering `Pool::acquire` | — |
| `PoolAcquireFinish` | Connection handed to caller | `wait_duration`, `pool_size`, `idle` |
| `PoolRelease` | `PooledConnection` dropped (connection returned or discarded) | `reused` |

### Notifications

| Arm | Fires when | Key fields |
|-----|-----------|------------|
| `Notice` | **(future)** — declared but not yet emitted in-band during queries (see Limitations) | `severity`, `message`, `code` |
| `Notification` | LISTEN/NOTIFY delivery via `wait_for_notification` | `channel`, `payload` |

### Sentinel-level events (handled by `SntlTracing`)

| Arm | Fires when | Key fields |
|-----|-----------|------------|
| `QueryMacro` | Any `sntl::query!()` family call entering `fetch_*` / `execute` | `macro_name` (e.g. `"query_scalar!"`), `query_id` (13-char cache hash), `sql` |
| `ReducerBegin` | **(future)** — declared for `#[reducer]` macro; not yet emitted | `name` |
| `ReducerCommit` | **(future)** — same | `name`, `duration` |
| `ReducerRollback` | **(future)** — same | `name`, `error` |
| `MigrationApply` | `sntl_migrate::Migrator::run` — successful single migration | `version`, `duration`, `checksum` |
| `MigrationDrift` | `sntl_migrate::Migrator::info` — checksum mismatch on an applied migration | `version`, `recorded`, `current` |

---

## Limitations

| Limitation | Detail |
|-----------|--------|
| **Sync-only `on_event`** | `on_event` is a synchronous `&self` call. If you need to drive async I/O (e.g. publish to a queue), spawn a task from within the handler and pass the data via a channel. |
| **No per-event filter API at trait level** | Filtering is the adapter's responsibility. `TracingInstrumentation` respects the `tracing` subscriber's `EnvFilter`; custom adapters must implement their own if/else logic. |
| **In-band `Notice` events not yet emitted** | PostgreSQL `NOTICE` / `WARNING` messages received during query execution are not forwarded to `on_event` in the current driver. This is a known driver limitation tracked for a post-3.0 release. |
| **`cache_hit` always `false`** | The `PrepareFinish.cache_hit` field is always `false` today because prepared-statement cache wiring is not yet complete in the driver. |
| **`Reducer*` events dormant** | `ReducerBegin`, `ReducerCommit`, and `ReducerRollback` are declared in `Event` and handled by `SntlTracing`, but the `#[reducer]` macro does not yet exist in `sntl-macros`. The handler arms are ready; they will activate once the macro lands. |
| **`Connect` / `Authenticated` / `Disconnect` not emitted** | These require pre-Connection plumbing changes in the driver that are deferred post-3.0. |
