//! End-to-end OTel: install tracing-opentelemetry → run a query → assert
//! the exporter captured a span with db.system = postgresql.
//! Skips silently without DATABASE_URL.
//!
//! Design note: opentelemetry-stdout 0.32 has no `with_writer` / builder API
//! — it writes unconditionally to stdout. We implement a thin
//! `CapturingExporter` that stores `SpanData` in memory instead, which lets
//! us assert on attributes directly without any stdout redirection trickery.

use std::sync::{Arc, Mutex};

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::{SdkTracerProvider, SpanData, SpanExporter};
use tracing::Instrument;
use tracing_subscriber::layer::SubscriberExt;

// ---------------------------------------------------------------------------
// CapturingExporter — stores every exported SpanData for later inspection.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct CapturingExporter {
    spans: Arc<Mutex<Vec<SpanData>>>,
}

impl CapturingExporter {
    fn new() -> (Self, Arc<Mutex<Vec<SpanData>>>) {
        let store = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                spans: Arc::clone(&store),
            },
            store,
        )
    }
}

impl SpanExporter for CapturingExporter {
    fn export(
        &self,
        batch: Vec<SpanData>,
    ) -> impl std::future::Future<Output = opentelemetry_sdk::error::OTelSdkResult> + Send {
        let spans = Arc::clone(&self.spans);
        async move {
            spans.lock().unwrap().extend(batch);
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn query_exports_otel_span_with_db_system() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else {
        return;
    };

    let (exporter, captured) = CapturingExporter::new();

    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.tracer("sntl-otel-test");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // set_default is scoped to this thread and dropped with `_guard`,
    // so it does not conflict with other tests running concurrently.
    let subscriber = tracing_subscriber::registry().with(otel_layer);
    let _guard = tracing::subscriber::set_default(subscriber);

    let cfg = sntl::driver::Config::parse(&url)
        .unwrap()
        .with_instrumentation(Arc::new(sntl::driver::TracingInstrumentation::default()));
    let mut conn = sntl::driver::Connection::connect(cfg).await.unwrap();

    async {
        conn.query("SELECT 1::int4", &[]).await.unwrap();
    }
    .instrument(tracing::info_span!(
        "db.query",
        db.system = tracing::field::Empty,
        db.statement = tracing::field::Empty,
        db.operation = tracing::field::Empty,
        db.rows_affected = tracing::field::Empty,
        sntl.param_count = tracing::field::Empty,
        sntl.duration_us = tracing::field::Empty,
    ))
    .await;

    // Flush: drop the provider so the SimpleSpanProcessor flushes synchronously.
    drop(_guard);
    provider.shutdown().ok();

    let spans = captured.lock().unwrap();
    assert!(!spans.is_empty(), "OTel exporter captured no spans");

    let db_query_span = spans
        .iter()
        .find(|s| s.name.as_ref() == "db.query")
        .expect("expected a span named 'db.query'");

    let db_system = db_query_span
        .attributes
        .iter()
        .find(|kv| kv.key.as_str() == "db.system")
        .expect("expected db.system attribute on db.query span");

    assert_eq!(
        db_system.value.as_str(),
        "postgresql",
        "db.system must equal 'postgresql', got: {:?}",
        db_system.value,
    );
}
