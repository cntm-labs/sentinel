//! Integration tests: Pool + PooledConnection with query builders.
//!
//! Skipped when DATABASE_URL is not set.

#[macro_use]
mod pg_helpers;

use sntl::prelude::*;

#[tokio::test]
async fn pool_acquire_and_query() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let pool_config = sntl::driver::pool::config::PoolConfig::new().max_connections(2);
    let pool = Pool::new(config, pool_config);

    let mut conn = pool.acquire().await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // PooledConnection Derefs to Connection — query builders work directly
    InsertQuery::new("users")
        .column("name", "PoolUser")
        .column("email", "pool@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    let name: String = rows[0].get(1);
    assert_eq!(name, "PoolUser");
}

#[tokio::test]
async fn pool_transaction_through_pooled_connection() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let pool_config = sntl::driver::pool::config::PoolConfig::new().max_connections(2);
    let pool = Pool::new(config, pool_config);

    let mut conn = pool.acquire().await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Transaction works through PooledConnection via DerefMut
    let mut tx = Transaction::begin(&mut conn).await.unwrap();
    InsertQuery::new("users")
        .column("name", "TxPoolUser")
        .column("email", "txpool@test.com")
        .no_returning()
        .execute(tx.conn())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
}
