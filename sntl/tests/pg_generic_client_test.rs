//! Integration tests: GenericClient trait — queries via PooledConnection.
//!
//! Verifies that all query builders work through the GenericClient trait,
//! accepting both Connection and PooledConnection.

#[macro_use]
mod pg_helpers;

use sntl::prelude::*;

#[tokio::test]
async fn query_via_pooled_connection() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let pool = sntl::driver::Pool::new(config, Default::default());
    let mut conn = pool.acquire().await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // PooledConnection should work with all query builders
    InsertQuery::new("users")
        .column("name", "Pool User")
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
}

#[tokio::test]
async fn select_fetch_one_via_pool() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let pool = sntl::driver::Pool::new(config, Default::default());
    let mut conn = pool.acquire().await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    InsertQuery::new("users")
        .column("name", "One User")
        .column("email", "one@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    let row = SelectQuery::new("users")
        .where_(Column::new("users", "email").eq("one@test.com"))
        .fetch_one(&mut conn)
        .await
        .unwrap();

    let name: String = row.get_by_name("name");
    assert_eq!(name, "One User");
}
