//! Sentinel-level observability bridge.
//!
//! Wraps `sentinel_driver::TracingInstrumentation` and adds handlers for
//! Sentinel-specific events (`QueryMacro`, `Reducer*`, `Migration*`).
//! All driver-level events delegate to the wrapped impl.

use std::sync::Arc;

use driver::{Event, Instrumentation, TracingInstrumentation};

/// Observability adapter that handles Sentinel-level events and delegates
/// all driver-level events to the inner [`TracingInstrumentation`].
#[derive(Clone, Default)]
pub struct SntlTracing {
    inner: TracingInstrumentation,
}

impl SntlTracing {
    /// Wrap an existing [`TracingInstrumentation`] instance.
    pub fn with_inner(inner: TracingInstrumentation) -> Self {
        Self { inner }
    }
}

impl Instrumentation for SntlTracing {
    fn on_event(&self, ev: &Event<'_>) {
        let span = tracing::Span::current();
        match ev {
            Event::QueryMacro {
                macro_name,
                query_id,
                sql: _,
            } => {
                span.record("sntl.macro", macro_name);
                span.record("sntl.query_id", query_id);
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
            Event::MigrationApply {
                version,
                duration,
                checksum,
            } => {
                tracing::info!(
                    version = %version,
                    duration_us = duration.as_micros() as i64,
                    checksum = %checksum,
                    "migration applied"
                );
            }
            Event::MigrationDrift {
                version,
                recorded,
                current,
            } => {
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

/// Install `SntlTracing::default()` on the pool. Matches the consume-self
/// signature of `Pool::with_instrumentation`.
pub fn install_default_tracing(pool: driver::Pool) -> driver::Pool {
    pool.with_instrumentation(Arc::new(SntlTracing::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use driver::{AcquireOutcome, DisconnectReason, Event, Outcome, RollbackReason, StmtRef};
    use std::time::Duration;

    /// Drive every arm of `SntlTracing::on_event` so cargo-llvm-cov sees
    /// each branch executed at least once.
    #[test]
    fn synthetic_all_arms_execute() {
        let adapter = SntlTracing::default();
        let sql = "SELECT 1";

        // Sentinel-level events
        adapter.on_event(&Event::QueryMacro {
            macro_name: "query",
            query_id: "abc",
            sql,
        });
        adapter.on_event(&Event::ReducerBegin { name: "r1" });
        adapter.on_event(&Event::ReducerCommit {
            name: "r1",
            duration: Duration::from_micros(10),
        });
        adapter.on_event(&Event::ReducerRollback {
            name: "r1",
            error: "rolled back",
        });
        adapter.on_event(&Event::MigrationApply {
            version: "20260514_120000_init",
            duration: Duration::from_millis(5),
            checksum: "deadbeef",
        });
        adapter.on_event(&Event::MigrationDrift {
            version: "20260514_120000_init",
            recorded: "aaa",
            current: "bbb",
        });

        // Driver-level events that go through `_ => self.inner.on_event(ev)`
        adapter.on_event(&Event::ExecuteStart {
            stmt: StmtRef::Inline { sql },
            param_count: 0,
        });
        adapter.on_event(&Event::ExecuteFinish {
            stmt: StmtRef::Inline { sql },
            rows: 1,
            duration: Duration::from_micros(100),
            outcome: Outcome::Ok,
        });
        adapter.on_event(&Event::PrepareFinish {
            name: "stmt1",
            param_oids: &[],
            col_count: 1,
            duration: Duration::from_micros(50),
            cache_hit: false,
        });
        adapter.on_event(&Event::TxBegin { isolation: None });
        adapter.on_event(&Event::TxCommit {
            duration: Duration::from_micros(10),
        });
        adapter.on_event(&Event::TxRollback {
            duration: Duration::from_micros(10),
            reason: RollbackReason::Explicit,
        });
        adapter.on_event(&Event::PipelineFlush {
            batch_len: 5,
            total_duration: Duration::from_millis(2),
        });
        adapter.on_event(&Event::PoolAcquireFinish {
            wait: Duration::from_micros(20),
            outcome: AcquireOutcome::Ok,
        });
        adapter.on_event(&Event::Notice {
            severity: "NOTICE",
            code: "00000",
            message: "test notice",
        });
        adapter.on_event(&Event::Notification {
            channel: "ch",
            payload: "p",
            pid: 1234,
        });
        adapter.on_event(&Event::PoolRelease);
        adapter.on_event(&Event::Connect {
            host: "localhost",
            port: 5432,
        });
        adapter.on_event(&Event::Authenticated { user: "sentinel" });
        adapter.on_event(&Event::Disconnect {
            reason: DisconnectReason::Graceful,
        });
        adapter.on_event(&Event::PrepareStart { name: "stmt1", sql });
        adapter.on_event(&Event::PipelineStart { batch_len: 5 });
        adapter.on_event(&Event::PoolAcquireStart { pending: 0 });

        // Test `with_inner` constructor path
        let custom = SntlTracing::with_inner(driver::TracingInstrumentation::default());
        custom.on_event(&Event::QueryMacro {
            macro_name: "query",
            query_id: "abc",
            sql,
        });

        // Verify the Arc<dyn Instrumentation> upcast compiles
        let _arc: Arc<dyn Instrumentation> = Arc::new(SntlTracing::default());
    }
}
