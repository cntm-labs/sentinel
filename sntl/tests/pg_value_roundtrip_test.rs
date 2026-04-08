//! Integration tests: Value encode → PostgreSQL → Row decode roundtrip.
//!
//! Skipped when DATABASE_URL is not set.

#[macro_use]
mod pg_helpers;

use chrono::{TimeZone, Utc};
use sntl::prelude::*;
use uuid::Uuid;

// Helper: insert a single value into type_roundtrip and read it back.
async fn roundtrip_one(
    conn: &mut sntl::driver::Connection,
    col: &str,
    value: Value,
) -> sntl::driver::Row {
    pg_helpers::clean_tables(conn).await;

    let sql = format!("INSERT INTO \"type_roundtrip\" (\"{col}\") VALUES ($1) RETURNING *");
    let params: Vec<&(dyn sntl::driver::ToSql + Sync)> = vec![&value];
    let rows = conn.query(&sql, &params).await.unwrap();
    assert_eq!(rows.len(), 1);
    rows.into_iter().next().unwrap()
}

#[tokio::test]
async fn roundtrip_bool() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "bool_col", Value::Bool(true)).await;
    let val: bool = row.get(1);
    assert!(val);
}

#[tokio::test]
async fn roundtrip_int() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "int_col", Value::Int(42)).await;
    let val: i32 = row.get(2);
    assert_eq!(val, 42);
}

#[tokio::test]
async fn roundtrip_bigint() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "bigint_col", Value::BigInt(9_999_999_999)).await;
    let val: i64 = row.get(3);
    assert_eq!(val, 9_999_999_999);
}

#[tokio::test]
async fn roundtrip_double() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "double_col", Value::Double(42.195)).await;
    let val: f64 = row.get(4);
    assert!((val - 42.195).abs() < f64::EPSILON);
}

#[tokio::test]
async fn roundtrip_text() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(
        &mut conn,
        "text_col",
        Value::Text("hello sentinel".to_owned()),
    )
    .await;
    let val: String = row.get(5);
    assert_eq!(val, "hello sentinel");
}

#[tokio::test]
async fn roundtrip_uuid() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let id = Uuid::new_v4();
    let row = roundtrip_one(&mut conn, "uuid_col", Value::Uuid(id)).await;
    let val: Uuid = row.get(6);
    assert_eq!(val, id);
}

#[tokio::test]
async fn roundtrip_timestamptz() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let ts = Utc.with_ymd_and_hms(2026, 4, 8, 12, 0, 0).unwrap();
    let row = roundtrip_one(&mut conn, "timestamptz_col", Value::Timestamp(ts)).await;
    let val: chrono::DateTime<Utc> = row.get(7);
    assert_eq!(val, ts);
}

#[tokio::test]
async fn roundtrip_bytea() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let row = roundtrip_one(&mut conn, "bytea_col", Value::Bytes(data.clone())).await;
    let val: Vec<u8> = row.get(8);
    assert_eq!(val, data);
}

#[tokio::test]
async fn roundtrip_null() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "text_col", Value::Null).await;
    let val: Option<String> = row.try_get(5).unwrap();
    assert!(val.is_none());
}
