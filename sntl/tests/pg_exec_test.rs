//! Integration tests: query builder execution against live PostgreSQL.
//!
//! Skipped when DATABASE_URL is not set.

#[macro_use]
mod pg_helpers;

use sntl::prelude::*;

// ── INSERT ──────────────────────────────────────────────────────

#[tokio::test]
async fn insert_and_fetch_returning() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::truncate(&mut conn, "users").await;

    let rows = InsertQuery::new("users")
        .column("name", "Alice")
        .column("email", "alice@test.com")
        .column("active", true)
        .fetch_returning(&mut conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    let name: String = rows[0].get(1);
    assert_eq!(name, "Alice");
}

#[tokio::test]
async fn insert_execute_returns_row_count() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::truncate(&mut conn, "users").await;

    let count = InsertQuery::new("users")
        .column("name", "Bob")
        .column("email", "bob@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!(count, 1);
}

// ── SELECT ──────────────────────────────────────────────────────

#[tokio::test]
async fn select_fetch_all() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::truncate(&mut conn, "users").await;

    // Insert two rows
    InsertQuery::new("users")
        .column("name", "Alice")
        .column("email", "alice2@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();
    InsertQuery::new("users")
        .column("name", "Bob")
        .column("email", "bob2@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
}

#[tokio::test]
async fn select_fetch_one_with_where() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::truncate(&mut conn, "users").await;

    InsertQuery::new("users")
        .column("name", "Charlie")
        .column("email", "charlie@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    let col = Column::new("users", "email");
    let row = SelectQuery::new("users")
        .where_(col.eq("charlie@test.com"))
        .fetch_one(&mut conn)
        .await
        .unwrap();

    let name: String = row.get(1);
    assert_eq!(name, "Charlie");
}

#[tokio::test]
async fn select_fetch_optional_returns_none() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::truncate(&mut conn, "users").await;

    let col = Column::new("users", "email");
    let row = SelectQuery::new("users")
        .where_(col.eq("nonexistent@test.com"))
        .fetch_optional(&mut conn)
        .await
        .unwrap();

    assert!(row.is_none());
}

// ── UPDATE ──────────────────────────────────────────────────────

#[tokio::test]
async fn update_execute_modifies_rows() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::truncate(&mut conn, "users").await;

    InsertQuery::new("users")
        .column("name", "Dave")
        .column("email", "dave@test.com")
        .column("active", true)
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    let col = Column::new("users", "email");
    let affected = UpdateQuery::new("users")
        .set("active", false)
        .where_(col.eq("dave@test.com"))
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!(affected, 1);

    // Verify the update
    let row = SelectQuery::new("users")
        .where_(col.eq("dave@test.com"))
        .fetch_one(&mut conn)
        .await
        .unwrap();

    let active: bool = row.get(3);
    assert!(!active);
}

// ── DELETE ───────────────────────────────────────────────────────

#[tokio::test]
async fn delete_removes_rows() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::truncate(&mut conn, "users").await;

    InsertQuery::new("users")
        .column("name", "Eve")
        .column("email", "eve@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    let col = Column::new("users", "email");
    let affected = DeleteQuery::new("users")
        .where_(col.eq("eve@test.com"))
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!(affected, 1);

    let rows = SelectQuery::new("users")
        .fetch_all(&mut conn)
        .await
        .unwrap();

    assert!(rows.is_empty());
}
