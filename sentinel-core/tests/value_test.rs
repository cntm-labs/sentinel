use chrono::{TimeZone, Utc};
use sentinel_core::types::Value;
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
    let v: Value = 3.14f64.into();
    assert!(matches!(v, Value::Double(f) if (f - 3.14).abs() < f64::EPSILON));
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
