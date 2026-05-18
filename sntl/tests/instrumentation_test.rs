//! Live-PG verification that sntl-level events fire from query!() macros.
//! Skips silently without DATABASE_URL.

use std::sync::{Arc, Mutex};

use sntl::driver::{Event, Instrumentation};

#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        if let Event::QueryMacro {
            macro_name,
            query_id,
            ..
        } = ev
        {
            self.0
                .lock()
                .unwrap()
                .push(format!("{macro_name}:{query_id}"));
        }
    }
}

#[tokio::test]
async fn query_macro_fires_with_id() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else {
        return;
    };
    let rec = Arc::new(Recorder::default());
    let cfg = sntl::driver::Config::parse(&url)
        .unwrap()
        .with_instrumentation(rec.clone());
    let conn = sntl::driver::Connection::connect(cfg).await.unwrap();

    let _: i32 = sntl::query_scalar!("SELECT 1::int4")
        .fetch_one(conn)
        .await
        .unwrap();

    let evs = rec.0.lock().unwrap();
    assert!(!evs.is_empty(), "QueryMacro must fire");
    assert!(
        evs[0].starts_with("query_scalar:"),
        "expected query_scalar prefix, got: {}",
        evs[0]
    );
}
