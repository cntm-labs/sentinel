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
