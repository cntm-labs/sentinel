use sntl::core::types::Value;
use sntl::driver::Oid;
use sntl::driver::types::ToSql;

#[test]
fn value_bool_to_sql() {
    let v = Value::Bool(true);
    assert_eq!(v.oid(), Oid::BOOL);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &[1u8]); // PG binary: true = 0x01
}

#[test]
fn value_int_to_sql() {
    let v = Value::Int(42);
    assert_eq!(v.oid(), Oid::INT4);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &42i32.to_be_bytes());
}

#[test]
fn value_bigint_to_sql() {
    let v = Value::BigInt(123456789);
    assert_eq!(v.oid(), Oid::INT8);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &123456789i64.to_be_bytes());
}

#[test]
fn value_text_to_sql() {
    let v = Value::Text("hello".into());
    assert_eq!(v.oid(), Oid::TEXT);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), b"hello");
}

#[test]
fn value_double_to_sql() {
    let v = Value::Double(1.23);
    assert_eq!(v.oid(), Oid::FLOAT8);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &1.23f64.to_be_bytes());
}

#[test]
fn value_uuid_to_sql() {
    let id = uuid::Uuid::nil();
    let v = Value::Uuid(id);
    assert_eq!(v.oid(), Oid::UUID);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 16);
}

#[test]
fn value_bytes_to_sql() {
    let v = Value::Bytes(vec![0x01, 0x02, 0x03]);
    assert_eq!(v.oid(), Oid::BYTEA);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &[0x01, 0x02, 0x03]);
}

#[test]
fn value_timestamp_to_sql() {
    let ts = chrono::Utc::now();
    let v = Value::Timestamp(ts);
    assert_eq!(v.oid(), Oid::TIMESTAMPTZ);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 8); // PG binary: i64 microseconds from J2000
}

#[test]
fn value_null_to_sql() {
    let v = Value::Null;
    assert_eq!(v.oid(), Oid::TEXT); // default OID for NULL
    assert!(v.is_null());
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(buf.is_empty()); // NULL writes nothing; driver sends -1 length via is_null()
}

#[test]
fn value_non_null_is_not_null() {
    assert!(!Value::Bool(true).is_null());
    assert!(!Value::Int(0).is_null());
    assert!(!Value::Text(String::new()).is_null());
}

// === New variant ToSql tests ===

#[test]
fn value_smallint_to_sql() {
    let v = Value::SmallInt(42);
    assert_eq!(v.oid(), Oid::INT2);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &42i16.to_be_bytes());
}

#[test]
fn value_float_to_sql() {
    let v = Value::Float(1.5);
    assert_eq!(v.oid(), Oid::FLOAT4);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &1.5f32.to_be_bytes());
}

#[test]
fn value_json_to_sql() {
    let v = Value::Json(serde_json::json!({"key": "val"}));
    assert_eq!(v.oid(), Oid::JSONB);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    // JSONB binary format: 1-byte version prefix (0x01) + JSON text
    assert_eq!(buf[0], 1u8);
    let json_text = std::str::from_utf8(&buf[1..]).unwrap();
    assert!(json_text.contains("key"));
}

#[test]
fn value_date_to_sql() {
    let d = chrono::NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
    let v = Value::Date(d);
    assert_eq!(v.oid(), Oid::DATE);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 4); // PG DATE is i32 (days from J2000)
}

#[test]
fn value_time_to_sql() {
    let t = chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    let v = Value::Time(t);
    assert_eq!(v.oid(), Oid::TIME);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 8); // PG TIME is i64 microseconds
}

#[test]
fn value_inet_to_sql() {
    let v = Value::Inet(std::net::Ipv4Addr::LOCALHOST.into());
    assert_eq!(v.oid(), Oid::INET);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn value_interval_to_sql() {
    use sntl::driver::types::interval::PgInterval;
    let v = Value::Interval(PgInterval {
        months: 1,
        days: 2,
        microseconds: 3_000_000,
    });
    assert_eq!(v.oid(), Oid::INTERVAL);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 16);
}

#[test]
fn value_point_to_sql() {
    use sntl::driver::types::geometric::PgPoint;
    let v = Value::Point(PgPoint { x: 1.0, y: 2.0 });
    assert_eq!(v.oid(), Oid::POINT);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 16);
}

#[test]
fn value_money_to_sql() {
    let v = Value::Money(12345);
    assert_eq!(v.oid(), Oid::MONEY);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &12345i64.to_be_bytes());
}

#[test]
fn value_macaddr_to_sql() {
    let v = Value::MacAddr([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    assert_eq!(v.oid(), Oid::MACADDR);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
}

#[test]
fn value_int_array_to_sql() {
    let v = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    assert_eq!(v.oid(), Oid::INT4_ARRAY);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn value_text_array_to_sql() {
    let v = Value::Array(vec![Value::Text("a".into()), Value::Text("b".into())]);
    assert_eq!(v.oid(), Oid::TEXT_ARRAY);
}

#[test]
fn value_empty_array_errors() {
    let v = Value::Array(vec![]);
    let mut buf = bytes::BytesMut::new();
    assert!(v.to_sql(&mut buf).is_err());
}
