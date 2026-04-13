use chrono::{TimeZone, Utc};
use sntl::core::types::Value;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

#[test]
fn value_from_string() {
    let v: Value = "hello".to_string().into();
    assert!(matches!(v, Value::Text(s) if s == "hello"));
}

#[test]
fn value_from_str() {
    let v: Value = Value::from("hello");
    assert!(matches!(v, Value::Text(s) if s == "hello"));
}

#[test]
fn value_from_i64() {
    let v: Value = 42i64.into();
    assert!(matches!(v, Value::BigInt(42)));
}

#[test]
fn value_from_i32() {
    let v: Value = 42i32.into();
    assert!(matches!(v, Value::Int(42)));
}

#[test]
fn value_from_bool() {
    let v: Value = true.into();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn value_from_f64() {
    let v: Value = 2.72f64.into();
    assert!(matches!(v, Value::Double(f) if (f - 2.72).abs() < f64::EPSILON));
}

#[test]
fn value_from_uuid() {
    let id = Uuid::new_v4();
    let v: Value = id.into();
    assert!(matches!(v, Value::Uuid(u) if u == id));
}

#[test]
fn value_from_datetime() {
    let dt = Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap();
    let v: Value = dt.into();
    assert!(matches!(v, Value::Timestamp(d) if d == dt));
}

#[test]
fn value_null() {
    let v = Value::Null;
    assert!(matches!(v, Value::Null));
}

#[test]
fn value_from_option_some() {
    let v: Value = Some(42i64).into();
    assert!(matches!(v, Value::BigInt(42)));
}

#[test]
fn value_from_option_none() {
    let v: Value = Option::<i64>::None.into();
    assert!(matches!(v, Value::Null));
}

#[test]
fn value_from_bytes() {
    let v: Value = vec![0x01u8, 0x02, 0x03].into();
    assert!(matches!(v, Value::Bytes(b) if b == vec![0x01, 0x02, 0x03]));
}

// === New scalar From impls ===

#[test]
fn value_from_i16() {
    let v: Value = 42i16.into();
    assert!(matches!(v, Value::SmallInt(42)));
}

#[test]
fn value_from_f32() {
    let v: Value = 1.5f32.into();
    assert!(matches!(v, Value::Float(f) if (f - 1.5).abs() < f32::EPSILON));
}

#[test]
fn value_from_serde_json() {
    let j = serde_json::json!({"key": "val"});
    let v: Value = j.clone().into();
    assert!(matches!(v, Value::Json(ref inner) if inner == &j));
}

#[test]
fn value_from_ipaddr() {
    let ip: IpAddr = Ipv4Addr::LOCALHOST.into();
    let v: Value = ip.into();
    assert!(matches!(v, Value::Inet(addr) if addr == ip));
}

#[test]
fn value_from_naive_date() {
    let d = chrono::NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
    let v: Value = d.into();
    assert!(matches!(v, Value::Date(inner) if inner == d));
}

#[test]
fn value_from_naive_time() {
    let t = chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    let v: Value = t.into();
    assert!(matches!(v, Value::Time(inner) if inner == t));
}

#[test]
fn value_from_naive_datetime() {
    let dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 13)
        .unwrap()
        .and_hms_opt(14, 30, 0)
        .unwrap();
    let v: Value = dt.into();
    assert!(matches!(v, Value::TimestampNaive(inner) if inner == dt));
}

#[test]
fn value_from_decimal() {
    let d = rust_decimal::Decimal::new(12345, 2); // 123.45
    let v: Value = d.into();
    assert!(matches!(v, Value::Numeric(inner) if inner == d));
}

// === Complex type constructors ===

#[test]
fn value_interval() {
    let v = Value::Interval(driver::types::interval::PgInterval {
        months: 1,
        days: 2,
        microseconds: 3_000_000,
    });
    assert!(matches!(v, Value::Interval(_)));
}

#[test]
fn value_point() {
    let v = Value::Point(driver::types::geometric::PgPoint { x: 1.0, y: 2.0 });
    assert!(matches!(v, Value::Point(_)));
}

#[test]
fn value_array_homogeneous() {
    let v = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    assert!(matches!(v, Value::Array(ref arr) if arr.len() == 3));
}

#[test]
fn value_macaddr() {
    let v = Value::MacAddr([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    assert!(matches!(v, Value::MacAddr(m) if m == [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]));
}
