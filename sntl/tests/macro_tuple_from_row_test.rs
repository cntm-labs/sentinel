//! Live-PG integration test for plain tuple FromRow target. Skips silently
//! when DATABASE_URL is unset.

#[macro_use]
mod pg_helpers;

use sntl::driver::{Config, Connection};

#[tokio::test]
async fn tuple_query_as_returns_tuple_directly() {
    let url = require_pg!();
    let mut conn = Connection::connect(Config::parse(&url).expect("parse DATABASE_URL"))
        .await
        .expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    let inserted_id: i32 = conn
        .query_one(
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id",
            &[&"tuple-test", &"tuple@example.com"],
        )
        .await
        .unwrap()
        .try_get(0)
        .unwrap();

    let (id,): (i32,) = sntl::query_as!(
        (i32,),
        "SELECT id FROM users WHERE id = $1",
        inserted_id
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(id, inserted_id);
}
