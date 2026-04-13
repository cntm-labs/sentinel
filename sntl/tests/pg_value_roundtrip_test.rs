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

// === Existing roundtrips (migrated to get_by_name) ===

#[tokio::test]
async fn roundtrip_bool() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "bool_col", Value::Bool(true)).await;
    let val: bool = row.get_by_name("bool_col");
    assert!(val);
}

#[tokio::test]
async fn roundtrip_int() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "int_col", Value::Int(42)).await;
    let val: i32 = row.get_by_name("int_col");
    assert_eq!(val, 42);
}

#[tokio::test]
async fn roundtrip_bigint() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "bigint_col", Value::BigInt(9_999_999_999)).await;
    let val: i64 = row.get_by_name("bigint_col");
    assert_eq!(val, 9_999_999_999);
}

#[tokio::test]
async fn roundtrip_double() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "double_col", Value::Double(42.195)).await;
    let val: f64 = row.get_by_name("double_col");
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
    let val: String = row.get_by_name("text_col");
    assert_eq!(val, "hello sentinel");
}

#[tokio::test]
async fn roundtrip_uuid() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let id = Uuid::new_v4();
    let row = roundtrip_one(&mut conn, "uuid_col", Value::Uuid(id)).await;
    let val: Uuid = row.get_by_name("uuid_col");
    assert_eq!(val, id);
}

#[tokio::test]
async fn roundtrip_timestamptz() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let ts = Utc.with_ymd_and_hms(2026, 4, 8, 12, 0, 0).unwrap();
    let row = roundtrip_one(&mut conn, "timestamptz_col", Value::Timestamp(ts)).await;
    let val: chrono::DateTime<Utc> = row.get_by_name("timestamptz_col");
    assert_eq!(val, ts);
}

#[tokio::test]
async fn roundtrip_bytea() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let row = roundtrip_one(&mut conn, "bytea_col", Value::Bytes(data.clone())).await;
    let val: Vec<u8> = row.get_by_name("bytea_col");
    assert_eq!(val, data);
}

#[tokio::test]
async fn roundtrip_null() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "text_col", Value::Null).await;
    let val: Option<String> = row.try_get_by_name("text_col").unwrap();
    assert!(val.is_none());
}

// === New scalar roundtrips ===

#[tokio::test]
async fn roundtrip_smallint() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "smallint_col", Value::SmallInt(42)).await;
    let val: i16 = row.get_by_name("smallint_col");
    assert_eq!(val, 42);
}

#[tokio::test]
async fn roundtrip_float() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let row = roundtrip_one(&mut conn, "float_col", Value::Float(1.5)).await;
    let val: f32 = row.get_by_name("float_col");
    assert!((val - 1.5).abs() < f32::EPSILON);
}

#[tokio::test]
async fn roundtrip_numeric() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let d = rust_decimal::Decimal::new(12345, 2); // 123.45
    let row = roundtrip_one(&mut conn, "numeric_col", Value::Numeric(d)).await;
    let val: rust_decimal::Decimal = row.get_by_name("numeric_col");
    // NUMERIC(20,6) rounds to 6 decimal places → 123.450000
    assert_eq!(val.to_string(), "123.450000");
}

#[tokio::test]
async fn roundtrip_date() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let d = chrono::NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
    let row = roundtrip_one(&mut conn, "date_col", Value::Date(d)).await;
    let val: chrono::NaiveDate = row.get_by_name("date_col");
    assert_eq!(val, d);
}

#[tokio::test]
async fn roundtrip_time() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let t = chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    let row = roundtrip_one(&mut conn, "time_col", Value::Time(t)).await;
    let val: chrono::NaiveTime = row.get_by_name("time_col");
    assert_eq!(val, t);
}

#[tokio::test]
async fn roundtrip_timestamp_naive() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 13)
        .unwrap()
        .and_hms_opt(14, 30, 0)
        .unwrap();
    let row = roundtrip_one(&mut conn, "timestamp_col", Value::TimestampNaive(dt)).await;
    let val: chrono::NaiveDateTime = row.get_by_name("timestamp_col");
    assert_eq!(val, dt);
}

#[tokio::test]
async fn roundtrip_inet() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let ip: std::net::IpAddr = std::net::Ipv4Addr::new(192, 168, 1, 1).into();
    let row = roundtrip_one(&mut conn, "inet_col", Value::Inet(ip)).await;
    let val: std::net::IpAddr = row.get_by_name("inet_col");
    assert_eq!(val, ip);
}

#[tokio::test]
async fn roundtrip_macaddr() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    let row = roundtrip_one(&mut conn, "macaddr_col", Value::MacAddr(mac)).await;
    let val: sntl::driver::types::network::PgMacAddr = row.get_by_name("macaddr_col");
    assert_eq!(val.0, mac);
}

#[tokio::test]
async fn roundtrip_interval() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    use sntl::driver::types::interval::PgInterval;
    let iv = PgInterval {
        months: 1,
        days: 15,
        microseconds: 3_600_000_000,
    };
    let row = roundtrip_one(&mut conn, "interval_col", Value::Interval(iv)).await;
    let val: PgInterval = row.get_by_name("interval_col");
    assert_eq!(val, iv);
}

// === Geometric roundtrips ===

#[tokio::test]
async fn roundtrip_point() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    use sntl::driver::types::geometric::PgPoint;
    let pt = PgPoint { x: 1.5, y: 2.5 };
    let row = roundtrip_one(&mut conn, "point_col", Value::Point(pt)).await;
    let val: PgPoint = row.get_by_name("point_col");
    assert_eq!(val, pt);
}

#[tokio::test]
async fn roundtrip_circle() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    use sntl::driver::types::geometric::{PgCircle, PgPoint};
    let c = PgCircle {
        center: PgPoint { x: 0.0, y: 0.0 },
        radius: 5.0,
    };
    let row = roundtrip_one(&mut conn, "circle_col", Value::Circle(c)).await;
    let val: PgCircle = row.get_by_name("circle_col");
    assert_eq!(val, c);
}

// === JSONB roundtrip ===

#[tokio::test]
async fn roundtrip_jsonb() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    let j = serde_json::json!({"key": "value", "num": 42});
    let row = roundtrip_one(&mut conn, "jsonb_col", Value::Json(j.clone())).await;
    // Driver doesn't have serde_json FromSql, so read as raw bytes and verify
    let raw: Vec<u8> = row.get_by_name("jsonb_col");
    // JSONB binary: first byte is version (0x01), rest is JSON text
    assert_eq!(raw[0], 1u8);
    let parsed: serde_json::Value = serde_json::from_slice(&raw[1..]).unwrap();
    assert_eq!(parsed, j);
}

// === Money roundtrip ===

#[tokio::test]
async fn roundtrip_money() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();

    use sntl::driver::types::money::PgMoney;
    let row = roundtrip_one(&mut conn, "money_col", Value::Money(12345)).await;
    let val: PgMoney = row.get_by_name("money_col");
    assert_eq!(val.0, 12345);
}
