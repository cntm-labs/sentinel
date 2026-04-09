//! Integration tests: PascalCase queries and batch loading against live PostgreSQL.
//!
//! Skipped when DATABASE_URL is not set.

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

#[tokio::test]
async fn pascal_find_fetch_all() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert test data
    InsertQuery::new("users")
        .column("name", "Alice")
        .column("email", "alice@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    let rows = User::Find().FetchAll(&mut conn).await.unwrap();
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn pascal_find_id_fetch_one() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert and get the ID
    let rows = InsertQuery::new("users")
        .column("name", "Bob")
        .column("email", "bob@test.com")
        .fetch_returning(&mut conn)
        .await
        .unwrap();
    let user_id: i32 = rows[0].get(0);

    let row = User::FindId(user_id).FetchOne(&mut conn).await.unwrap();
    let name: String = row.get(1);
    assert_eq!(name, "Bob");
}

#[tokio::test]
async fn pascal_find_with_where_and_limit() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert multiple users
    for (name, email) in [
        ("A", "a@test.com"),
        ("B", "b@test.com"),
        ("C", "c@test.com"),
    ] {
        InsertQuery::new("users")
            .column("name", name)
            .column("email", email)
            .no_returning()
            .execute(&mut conn)
            .await
            .unwrap();
    }

    let rows = User::Find()
        .OrderBy(User::NAME.asc())
        .Limit(2)
        .FetchAll(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let first_name: String = rows[0].get(1);
    assert_eq!(first_name, "A");
}

#[tokio::test]
async fn batch_load_posts_for_user() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert user
    let user_rows = InsertQuery::new("users")
        .column("name", "Alice")
        .column("email", "alice@test.com")
        .fetch_returning(&mut conn)
        .await
        .unwrap();
    let user_id: i32 = user_rows[0].get(0);

    // Insert posts
    for title in ["Post 1", "Post 2"] {
        InsertQuery::new("posts")
            .column("user_id", user_id)
            .column("title", title)
            .no_returning()
            .execute(&mut conn)
            .await
            .unwrap();
    }

    // Batch load via RelationSpec
    let spec = User::POSTS;
    let (sql, binds) = spec.build_batch_sql(&[user_id.into()]);
    let params: Vec<&(dyn sntl::driver::ToSql + Sync)> = binds
        .iter()
        .map(|v| v as &(dyn sntl::driver::ToSql + Sync))
        .collect();
    let post_rows = conn.query(&sql, &params).await.unwrap();
    assert_eq!(post_rows.len(), 2);
}

#[tokio::test]
async fn batch_load_with_filter() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert user + posts (one published, one not)
    let user_rows = InsertQuery::new("users")
        .column("name", "Alice")
        .column("email", "alice@test.com")
        .fetch_returning(&mut conn)
        .await
        .unwrap();
    let user_id: i32 = user_rows[0].get(0);

    InsertQuery::new("posts")
        .column("user_id", user_id)
        .column("title", "Published")
        .column("published", true)
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    InsertQuery::new("posts")
        .column("user_id", user_id)
        .column("title", "Draft")
        .column("published", false)
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    // Batch load only published posts
    let spec = User::POSTS.Filter(Post::PUBLISHED.eq(true));
    let (sql, binds) = spec.build_batch_sql(&[user_id.into()]);
    let params: Vec<&(dyn sntl::driver::ToSql + Sync)> = binds
        .iter()
        .map(|v| v as &(dyn sntl::driver::ToSql + Sync))
        .collect();
    let post_rows = conn.query(&sql, &params).await.unwrap();
    assert_eq!(post_rows.len(), 1);
    let title: String = post_rows[0].get(2);
    assert_eq!(title, "Published");
}
