//! Sentinel-level observability bridge.
//!
//! Wraps `sentinel_driver::TracingInstrumentation` and adds handlers for
//! Sentinel-specific events (`QueryMacro`, `Reducer*`, `Migration*`).
//! All driver-level events delegate to the wrapped impl.

use std::sync::Arc;

use driver::{Event, Instrumentation, TracingInstrumentation};

/// Observability adapter that handles Sentinel-level events and delegates
/// all driver-level events to the inner [`TracingInstrumentation`].
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
