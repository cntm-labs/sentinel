//! Live-PG verification that sntl-migrate emits MigrationApply per
//! migration and MigrationDrift when a file is tampered with.
//! Skips silently without DATABASE_URL.

use std::sync::{Arc, Mutex};

use sentinel_driver::pool::config::PoolConfig;
use sentinel_driver::{Event, Instrumentation};
use sntl_migrate::Migrator;
use tempfile::tempdir;

#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        let tag = match ev {
            Event::MigrationApply {
                version, checksum, ..
            } => {
                format!("apply:{version}:{checksum}")
            }
            Event::MigrationDrift { version, .. } => format!("drift:{version}"),
            _ => return,
        };
        self.0.lock().unwrap().push(tag);
    }
}

#[tokio::test]
async fn apply_then_drift() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else {
        return;
    };
    let rec = Arc::new(Recorder::default());
    let cfg = sentinel_driver::Config::parse(&url)
        .unwrap()
        .with_instrumentation(rec.clone());
    let pool = sentinel_driver::Pool::new(cfg, PoolConfig::new().max_connections(2));

    // Clean slate
    let mut admin = pool.acquire().await.unwrap();
    admin
        .execute("SET client_min_messages = ERROR", &[])
        .await
        .unwrap();
    admin
        .execute("DROP TABLE IF EXISTS _sntl_migrations, mig_instr_t", &[])
        .await
        .unwrap();
    drop(admin);

    let dir = tempdir().unwrap();
    let mig = dir.path().join("migrations/20260514_120000_create");
    std::fs::create_dir_all(&mig).unwrap();
    std::fs::write(mig.join("up.sql"), "CREATE TABLE mig_instr_t (id int);").unwrap();

    Migrator::from_dir(dir.path().join("migrations"))
        .unwrap()
        .run(&pool)
        .await
        .unwrap();

    // Tamper
    std::fs::write(
        mig.join("up.sql"),
        "CREATE TABLE mig_instr_t (id int); -- drifted",
    )
    .unwrap();
    Migrator::from_dir(dir.path().join("migrations"))
        .unwrap()
        .info(&pool)
        .await
        .unwrap();

    let evs = rec.0.lock().unwrap();
    assert!(
        evs.iter()
            .any(|e| e.starts_with("apply:20260514_120000_create")),
        "expected apply event for 20260514_120000_create, got: {:?}",
        *evs
    );
    assert!(
        evs.iter()
            .any(|e| e.starts_with("drift:20260514_120000_create")),
        "expected drift event for 20260514_120000_create, got: {:?}",
        *evs
    );
}
