//! Integration tests: Include query compilation against live PostgreSQL.
//!
//! Skipped when DATABASE_URL is not set.
//!
//! Phase 5B-1 validates compile-time safety. Full Include→FetchOne→accessor
//! execution path requires macro-generated decode bridges (Phase 5B-2).

#[macro_use]
mod pg_helpers;

use sntl::core::relation::HasMany;
use sntl::prelude::*;
use sntl::{Model, sentinel};

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Model)]
#[sentinel(table = "posts")]
pub struct Post {
    #[sentinel(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub published: bool,
}

#[sentinel(relations)]
impl User {
    pub fn posts() -> HasMany<Post> {
        HasMany::new("user_id")
    }
}

/// Validates that Include compiles and transitions types correctly.
#[tokio::test]
async fn include_compiles_and_builds_sql() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut _conn = sntl::driver::Connection::connect(config).await.unwrap();

    // Type-state chain compiles correctly
    let q = User::Find().Include(User::Posts());
    let (sql, _) = q.Build();
    assert!(sql.contains("users"));

    // With Where clause
    let q = User::FindId(1).Include(User::Posts());
    let (sql, binds) = q.Build();
    assert!(sql.contains("WHERE"));
    assert_eq!(binds.len(), 1);
}

/// Validates that FetchOne on IncludeQuery executes against PG.
/// Relation rows are stored as Vec<Row> — accessor decode is Phase 5B-2.
#[tokio::test]
async fn include_fetch_one_executes() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Seed data
    InsertQuery::new("users")
        .column("name", "Alice")
        .column("email", "alice@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();
    InsertQuery::new("posts")
        .column("user_id", 1)
        .column("title", "Post 1")
        .column("published", true)
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    // FetchOne with Include executes and returns WithRelations
    let user = User::FindId(1)
        .Include(User::Posts())
        .FetchOne(&mut conn)
        .await
        .unwrap();

    // Model fields via Deref
    assert_eq!(user.name, "Alice");
    assert_eq!(user.id, 1);

    // Relation data is stored as raw Vec<Row> — can verify it exists
    assert!(!user.relations().is_empty());
}
