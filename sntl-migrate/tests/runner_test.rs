//! Live-PG tests for `Migrator::run`. Skip silently when DATABASE_URL is unset.

use std::fs;
use std::path::Path;

use sentinel_driver::pool::config::PoolConfig;
use sntl_migrate::Migrator;
use tempfile::tempdir;

async fn pool() -> Option<sentinel_driver::Pool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = sentinel_driver::Config::parse(&url).ok()?;
    Some(sentinel_driver::Pool::new(
        cfg,
        PoolConfig::new().max_connections(4),
    ))
}

fn write_mig(root: &Path, version: &str, sql: &str) {
    let dir = root.join("migrations").join(version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("up.sql"), sql).unwrap();
}

async fn clean(pool: &sentinel_driver::Pool, sql: &str) {
    let mut conn = pool.acquire().await.unwrap();
    // Suppress NOTICEs so `DROP TABLE IF EXISTS` on a missing table doesn't
    // surface as a Protocol error in the driver's NoticeResponse handling.
    conn.execute("SET client_min_messages = ERROR", &[])
        .await
        .unwrap();
    conn.execute(sql, &[]).await.unwrap();
}

#[tokio::test]
async fn applies_pending_in_order_then_noop() {
    let Some(pool) = pool().await else { return };
    clean(&pool, "DROP TABLE IF EXISTS _sntl_migrations, runner_test").await;

    let dir = tempdir().unwrap();
    write_mig(
        dir.path(),
        "20260509_140000_create",
        "CREATE TABLE runner_test (id int);",
    );
    write_mig(
        dir.path(),
        "20260509_150000_insert",
        "INSERT INTO runner_test (id) VALUES (1);",
    );

    let migrator = Migrator::from_dir(dir.path().join("migrations")).unwrap();
    let first = migrator.run(&pool).await.unwrap();
    assert_eq!(first.applied.len(), 2);

    let second = migrator.run(&pool).await.unwrap();
    assert!(second.applied.is_empty(), "second run must be no-op");
}

#[tokio::test]
async fn out_of_order_errors() {
    let Some(pool) = pool().await else { return };
    clean(&pool, "DROP TABLE IF EXISTS _sntl_migrations").await;

    let dir = tempdir().unwrap();
    write_mig(dir.path(), "20260510_080000_b", "SELECT 1;");
    Migrator::from_dir(dir.path().join("migrations"))
        .unwrap()
        .run(&pool)
        .await
        .unwrap();

    // Now drop an earlier-timestamp migration into the same folder.
    write_mig(dir.path(), "20260509_080000_a", "SELECT 1;");
    let err = Migrator::from_dir(dir.path().join("migrations"))
        .unwrap()
        .run(&pool)
        .await
        .unwrap_err();
    assert!(matches!(err, sntl_migrate::Error::OutOfOrder { .. }));
}

#[tokio::test]
async fn notx_runs_outside_transaction() {
    let Some(pool) = pool().await else { return };
    clean(&pool, "DROP TABLE IF EXISTS _sntl_migrations, notx_test").await;
    clean(&pool, "CREATE TABLE notx_test (id int)").await;

    let dir = tempdir().unwrap();
    let mig = dir.path().join("migrations/20260509_140000_idx");
    fs::create_dir_all(&mig).unwrap();
    fs::write(
        mig.join("up.notx.sql"),
        "CREATE INDEX CONCURRENTLY notx_idx ON notx_test (id);",
    )
    .unwrap();
    Migrator::from_dir(dir.path().join("migrations"))
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
}
