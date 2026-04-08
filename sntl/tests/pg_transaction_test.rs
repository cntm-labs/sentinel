//! Integration tests: Transaction guard against live PostgreSQL.
//!
//! Skipped when DATABASE_URL is not set.

#[macro_use]
mod pg_helpers;

use sntl::prelude::*;

#[tokio::test]
async fn transaction_commit_persists() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert inside a committed transaction
    let mut tx = Transaction::begin(&mut conn).await.unwrap();
    tx.execute(
        "INSERT INTO \"users\" (\"name\", \"email\") VALUES ($1, $2)",
        &[
            &"TxUser" as &(dyn sntl::driver::ToSql + Sync),
            &"tx@test.com",
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Verify row persists after commit
    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn transaction_rollback_reverts() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert inside a rolled-back transaction
    let mut tx = Transaction::begin(&mut conn).await.unwrap();
    tx.execute(
        "INSERT INTO \"users\" (\"name\", \"email\") VALUES ($1, $2)",
        &[
            &"RollbackUser" as &(dyn sntl::driver::ToSql + Sync),
            &"rollback@test.com",
        ],
    )
    .await
    .unwrap();
    tx.rollback().await.unwrap();

    // Verify row does NOT persist
    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn transaction_drop_without_commit_reverts() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert inside a transaction that is dropped (not committed)
    {
        let mut tx = Transaction::begin(&mut conn).await.unwrap();
        tx.execute(
            "INSERT INTO \"users\" (\"name\", \"email\") VALUES ($1, $2)",
            &[
                &"DropUser" as &(dyn sntl::driver::ToSql + Sync),
                &"drop@test.com",
            ],
        )
        .await
        .unwrap();
        // tx dropped here without commit
    }

    // Connection is now in aborted transaction state.
    // We need to send ROLLBACK to reset it before querying.
    conn.rollback().await.unwrap();

    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn transaction_query_builder_through_conn() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Use typed query builders through tx.conn()
    let mut tx = Transaction::begin(&mut conn).await.unwrap();

    InsertQuery::new("users")
        .column("name", "QueryBuilderUser")
        .column("email", "qb@test.com")
        .no_returning()
        .execute(tx.conn())
        .await
        .unwrap();

    let rows = SelectQuery::new("users")
        .fetch_all(tx.conn())
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);

    tx.commit().await.unwrap();

    // Still visible after commit
    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn transaction_with_isolation_level() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    let mut tx = Transaction::begin_with(&mut conn, sntl::driver::IsolationLevel::Serializable)
        .await
        .unwrap();

    tx.execute(
        "INSERT INTO \"users\" (\"name\", \"email\") VALUES ($1, $2)",
        &[
            &"SerializableUser" as &(dyn sntl::driver::ToSql + Sync),
            &"serial@test.com",
        ],
    )
    .await
    .unwrap();

    tx.commit().await.unwrap();

    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
}
