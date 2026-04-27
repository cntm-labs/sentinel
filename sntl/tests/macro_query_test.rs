//! Live-PG integration test for `sntl::query!`. Skips silently when
//! DATABASE_URL is unset (mirrors every other pg_*.rs in this crate).

#[macro_use]
mod pg_helpers;

use sntl::driver::{Config, Connection};

#[tokio::test]
async fn query_macro_fetches_user_id() {
    let url = require_pg!();
    let mut conn = Connection::connect(Config::parse(&url).expect("parse DATABASE_URL"))
        .await
        .expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    let inserted_id: i32 = conn
        .query_one(
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id",
            &[&"Macro Test", &"macro@example.com"],
        )
        .await
        .unwrap()
        .try_get(0)
        .unwrap();

    // The cached query lives in .sentinel/queries/88f26472e41c0.json; the
    // schema snapshot in .sentinel/schema.toml mirrors setup.sql so the
    // proc-macro picks i32 for both the parameter and the column.
    let row = sntl::query!("SELECT id FROM users WHERE id = $1", inserted_id)
        .fetch_one(&mut conn)
        .await
        .expect("query!");
    assert_eq!(row.id, inserted_id);
}
