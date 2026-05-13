//! Live-PG tests for the tracking table. Skip silently when DATABASE_URL is unset.

use sntl_migrate::migration::Version;
use sntl_migrate::tracking;

async fn connect() -> Option<sentinel_driver::Connection> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = sentinel_driver::Config::parse(&url).ok()?;
    let mut conn = sentinel_driver::Connection::connect(cfg).await.ok()?;
    // Avoid NoticeResponse from `DROP TABLE IF EXISTS` against a missing table.
    conn.execute("SET client_min_messages = ERROR", &[])
        .await
        .ok()?;
    Some(conn)
}

#[tokio::test]
async fn ensure_is_idempotent() {
    let Some(mut conn) = connect().await else {
        return;
    };
    tracking::drop_table(&mut conn).await.ok();
    tracking::ensure(&mut conn).await.unwrap();
    tracking::ensure(&mut conn).await.unwrap();
}

#[tokio::test]
async fn record_and_load_round_trip() {
    let Some(mut conn) = connect().await else {
        return;
    };
    tracking::drop_table(&mut conn).await.ok();
    tracking::ensure(&mut conn).await.unwrap();
    let v: Version = "20260509_140000_a".parse().unwrap();
    tracking::record(&mut conn, &v, "abc123").await.unwrap();
    let applied = tracking::applied(&mut conn).await.unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].0, v);
    assert_eq!(applied[0].1, "abc123");
}
