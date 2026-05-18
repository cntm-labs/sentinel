//! Verify sntl::query!() / query_as!() / query_scalar!() accept &Pool,
//! Connection, and PooledConnection identically.
//! Live-PG; skips silently without DATABASE_URL.

use sntl::driver::pool::config::PoolConfig;
use sntl::driver::{Config, Pool};

async fn make_pool() -> Option<Pool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = Config::parse(&url).ok()?;
    Some(Pool::new(cfg, PoolConfig::new().max_connections(4)))
}

#[tokio::test]
async fn query_scalar_accepts_pool_ref() {
    let Some(pool) = make_pool().await else {
        return;
    };
    let n: i64 = sntl::query_scalar!("SELECT 42::int8")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 42);
}

#[tokio::test]
async fn query_scalar_accepts_connection() {
    let Some(pool) = make_pool().await else {
        return;
    };
    let conn = pool.acquire().await.unwrap();
    let n: i64 = sntl::query_scalar!("SELECT 42::int8")
        .fetch_one(conn)
        .await
        .unwrap();
    assert_eq!(n, 42);
}

#[tokio::test]
async fn query_accepts_pool_ref() {
    let Some(pool) = make_pool().await else {
        return;
    };
    let row = sntl::query!("SELECT 'alice'::text AS name, 42::int4 AS age")
        .fetch_one(&pool)
        .await
        .unwrap();
    let name: String = row.name;
    let age: i32 = row.age;
    assert_eq!(name, "alice");
    assert_eq!(age, 42);
}
