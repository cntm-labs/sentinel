use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A dynamically-typed SQL value used in query parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i32),
    BigInt(i64),
    Double(f64),
    Text(String),
    Uuid(Uuid),
    Timestamp(DateTime<Utc>),
    Bytes(Vec<u8>),
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_owned())
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::BigInt(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Double(v)
    }
}

impl From<Uuid> for Value {
    fn from(v: Uuid) -> Self {
        Value::Uuid(v)
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(v: DateTime<Utc>) -> Self {
        Value::Timestamp(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v)
    }
}

impl sentinel_driver::ToSql for Value {
    fn oid(&self) -> sentinel_driver::Oid {
        match self {
            Value::Null => sentinel_driver::Oid::TEXT,
            Value::Bool(_) => sentinel_driver::Oid::BOOL,
            Value::Int(_) => sentinel_driver::Oid::INT4,
            Value::BigInt(_) => sentinel_driver::Oid::INT8,
            Value::Double(_) => sentinel_driver::Oid::FLOAT8,
            Value::Text(_) => sentinel_driver::Oid::TEXT,
            Value::Uuid(_) => sentinel_driver::Oid::UUID,
            Value::Timestamp(_) => sentinel_driver::Oid::TIMESTAMPTZ,
            Value::Bytes(_) => sentinel_driver::Oid::BYTEA,
        }
    }

    fn to_sql(&self, buf: &mut bytes::BytesMut) -> sentinel_driver::Result<()> {
        use sentinel_driver::ToSql;
        match self {
            Value::Null => Ok(()),
            Value::Bool(v) => v.to_sql(buf),
            Value::Int(v) => v.to_sql(buf),
            Value::BigInt(v) => v.to_sql(buf),
            Value::Double(v) => v.to_sql(buf),
            Value::Text(v) => v.as_str().to_sql(buf),
            Value::Uuid(v) => v.to_sql(buf),
            Value::Timestamp(v) => v.to_sql(buf),
            Value::Bytes(v) => v.as_slice().to_sql(buf),
        }
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(inner) => inner.into(),
            None => Value::Null,
        }
    }
}
