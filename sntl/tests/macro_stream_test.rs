//! Verify sntl::query_unchecked!().into_stream().fetch_stream() yields rows
//! lazily and that closing the stream mid-iter leaves the connection usable.
//!
//! Live-PG integration test; skipped silently when DATABASE_URL is absent.

use sntl::driver::pool::config::PoolConfig;
use sntl::driver::{Config, GenericClient, Pool};

async fn make_pool() -> Option<Pool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = Config::parse(&url).ok()?;
    Some(Pool::new(cfg, PoolConfig::new().max_connections(2)))
}

async fn setup(conn: &mut impl GenericClient) {
    conn.execute("SET client_min_messages = ERROR", &[])
        .await
        .unwrap();
    conn.execute("DROP TABLE IF EXISTS stream_test", &[])
        .await
        .unwrap();
    conn.execute("CREATE TABLE stream_test (id int)", &[])
        .await
        .unwrap();
    conn.execute(
        "INSERT INTO stream_test (id) SELECT generate_series(1, 1000)",
        &[],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn streams_1000_rows_via_macro() {
    let Some(pool) = make_pool().await else {
        return;
    };

    let mut conn = pool.acquire().await.unwrap();
    setup(&mut *conn).await;

    let mut stream = sntl::query_unchecked!("SELECT id FROM stream_test ORDER BY id")
        .into_stream()
        .fetch_stream(&mut conn)
        .await
        .unwrap();

    let mut count = 0_i32;
    while let Some(row) = stream.next().await.unwrap() {
        let id: i32 = row.try_get(0).unwrap();
        count += 1;
        assert_eq!(id, count);
    }
    assert_eq!(count, 1000);
}

#[tokio::test]
async fn stream_close_mid_iter_leaves_conn_usable() {
    let Some(pool) = make_pool().await else {
        return;
    };

    let mut conn = pool.acquire().await.unwrap();
    setup(&mut *conn).await;

    {
        let mut stream = sntl::query_unchecked!("SELECT id FROM stream_test")
            .into_stream()
            .fetch_stream(&mut conn)
            .await
            .unwrap();
        let _first = stream.next().await.unwrap();
        stream.close().await.unwrap();
    }

    // Connection must be reusable after close().
    let row = conn
        .query_one("SELECT COUNT(*)::int8 FROM stream_test", &[])
        .await
        .unwrap();
    let n: i64 = row.try_get(0).unwrap();
    assert_eq!(n, 1000);
}
