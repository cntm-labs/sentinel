//! Live-PG integration tests for array element nullability. Skips silently
//! when DATABASE_URL is unset.
//!
//! Requires sentinel-driver >= 2.0.0 (Vec<Option<T>> Decode).

#[macro_use]
mod pg_helpers;

use sntl::driver::{Config, Connection};

#[tokio::test]
async fn array_with_null_elements_roundtrips() {
    let url = require_pg!();
    let mut conn = Connection::connect(Config::parse(&url).expect("parse"))
        .await
        .expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    conn.execute(
        "INSERT INTO users (name, email, tags) VALUES ($1, $2, ARRAY['a', NULL, 'b']::text[])",
        &[&"u1", &"u1@example.com"],
    )
    .await
    .unwrap();

    let row = sntl::query!("SELECT tags FROM users")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(
        row.tags,
        vec![Some("a".to_string()), None, Some("b".to_string()),]
    );
}

#[tokio::test]
async fn array_non_null_override_emits_vec_t() {
    let url = require_pg!();
    let mut conn = Connection::connect(Config::parse(&url).expect("parse"))
        .await
        .expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    conn.execute(
        "INSERT INTO users (name, email, tags) VALUES ($1, $2, ARRAY['a', 'b']::text[])",
        &[&"u2", &"u2@example.com"],
    )
    .await
    .unwrap();

    let row = sntl::query!("SELECT tags FROM users", non_null_elements = [tags])
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(row.tags, vec!["a".to_string(), "b".to_string()]);
}

#[tokio::test]
async fn array_non_null_override_errors_on_actual_null() {
    let url = require_pg!();
    let mut conn = Connection::connect(Config::parse(&url).expect("parse"))
        .await
        .expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    conn.execute(
        "INSERT INTO users (name, email, tags) VALUES ($1, $2, ARRAY['a', NULL]::text[])",
        &[&"u3", &"u3@example.com"],
    )
    .await
    .unwrap();

    // The auto-generated anonymous record struct does not derive Debug, so
    // .expect_err is unavailable; pattern-match on the result instead.
    let result = sntl::query!("SELECT tags FROM users", non_null_elements = [tags])
        .fetch_one(&mut conn)
        .await;
    match result {
        Ok(_) => panic!("decoding NULL into Vec<T> should have errored"),
        Err(err) => {
            let msg = format!("{err}");
            assert!(
                msg.contains("NULL elements not supported") || msg.contains("NULL"),
                "expected NULL-related decode error, got: {msg}"
            );
        }
    }
}
