use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use driver::ToSql;
use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

/// A dynamically-typed SQL value used in query parameters.
///
/// Covers all PostgreSQL types supported by sentinel-driver v1.0.0.
/// Complex driver types (PgInterval, PgPoint, etc.) are wrapped directly
/// rather than re-inventing encoding.
#[derive(Clone)]
pub enum Value {
    // === Existing scalars ===
    Null,
    Bool(bool),
    Int(i32),
    BigInt(i64),
    Double(f64),
    Text(String),
    Uuid(Uuid),
    Timestamp(DateTime<Utc>),
    Bytes(Vec<u8>),

    // === New scalars ===
    SmallInt(i16),
    Float(f32),
    Numeric(rust_decimal::Decimal),
    Money(i64),
    Xml(String),
    PgLsn(u64),
    Bit(driver::types::bit::PgBit),
    Json(serde_json::Value),

    // === Temporal ===
    Date(NaiveDate),
    Time(NaiveTime),
    TimeTz(driver::types::timetz::PgTimeTz),
    TimestampNaive(NaiveDateTime),

    // === Network ===
    Inet(IpAddr),
    Cidr(IpAddr),
    MacAddr([u8; 6]),
    MacAddr8(driver::types::network::PgMacAddr8),

    // === Interval ===
    Interval(driver::types::interval::PgInterval),

    // === Geometric ===
    Point(driver::types::geometric::PgPoint),
    Line(driver::types::geometric::PgLine),
    LineSegment(driver::types::geometric::PgLSeg),
    Box(driver::types::geometric::PgBox),
    Circle(driver::types::geometric::PgCircle),

    // === Extension types ===
    LTree(driver::types::ltree::PgLTree),
    LQuery(driver::types::ltree::PgLQuery),
    Cube(driver::types::cube::PgCube),

    // === Ranges ===
    Int4Range(driver::types::range::PgRange<i32>),
    Int8Range(driver::types::range::PgRange<i64>),
    NumRange(driver::types::range::PgRange<rust_decimal::Decimal>),
    TsRange(driver::types::range::PgRange<NaiveDateTime>),
    TsTzRange(driver::types::range::PgRange<DateTime<Utc>>),
    DateRange(driver::types::range::PgRange<NaiveDate>),

    // === Multiranges (PG 14+) ===
    Int4Multirange(driver::types::multirange::PgMultirange<i32>),
    Int8Multirange(driver::types::multirange::PgMultirange<i64>),
    NumMultirange(driver::types::multirange::PgMultirange<rust_decimal::Decimal>),
    TsMultirange(driver::types::multirange::PgMultirange<NaiveDateTime>),
    TsTzMultirange(driver::types::multirange::PgMultirange<DateTime<Utc>>),
    DateMultirange(driver::types::multirange::PgMultirange<NaiveDate>),

    // === Collections ===
    Array(Vec<Value>),

    // === Escape hatch for user-defined PG types ===
    Custom(Arc<dyn driver::ToSql + Send + Sync>),
}

// Manual Debug: dyn ToSql isn't Debug, so Custom prints as opaque.
impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Bool(v) => f.debug_tuple("Bool").field(v).finish(),
            Value::Int(v) => f.debug_tuple("Int").field(v).finish(),
            Value::BigInt(v) => f.debug_tuple("BigInt").field(v).finish(),
            Value::Double(v) => f.debug_tuple("Double").field(v).finish(),
            Value::Text(v) => f.debug_tuple("Text").field(v).finish(),
            Value::Uuid(v) => f.debug_tuple("Uuid").field(v).finish(),
            Value::Timestamp(v) => f.debug_tuple("Timestamp").field(v).finish(),
            Value::Bytes(v) => f.debug_tuple("Bytes").field(v).finish(),
            Value::SmallInt(v) => f.debug_tuple("SmallInt").field(v).finish(),
            Value::Float(v) => f.debug_tuple("Float").field(v).finish(),
            Value::Numeric(v) => f.debug_tuple("Numeric").field(v).finish(),
            Value::Money(v) => f.debug_tuple("Money").field(v).finish(),
            Value::Xml(v) => f.debug_tuple("Xml").field(v).finish(),
            Value::PgLsn(v) => f.debug_tuple("PgLsn").field(v).finish(),
            Value::Bit(v) => f.debug_tuple("Bit").field(v).finish(),
            Value::Json(v) => f.debug_tuple("Json").field(v).finish(),
            Value::Date(v) => f.debug_tuple("Date").field(v).finish(),
            Value::Time(v) => f.debug_tuple("Time").field(v).finish(),
            Value::TimeTz(v) => f.debug_tuple("TimeTz").field(v).finish(),
            Value::TimestampNaive(v) => f.debug_tuple("TimestampNaive").field(v).finish(),
            Value::Inet(v) => f.debug_tuple("Inet").field(v).finish(),
            Value::Cidr(v) => f.debug_tuple("Cidr").field(v).finish(),
            Value::MacAddr(v) => f.debug_tuple("MacAddr").field(v).finish(),
            Value::MacAddr8(v) => f.debug_tuple("MacAddr8").field(v).finish(),
            Value::Interval(v) => f.debug_tuple("Interval").field(v).finish(),
            Value::Point(v) => f.debug_tuple("Point").field(v).finish(),
            Value::Line(v) => f.debug_tuple("Line").field(v).finish(),
            Value::LineSegment(v) => f.debug_tuple("LineSegment").field(v).finish(),
            Value::Box(v) => f.debug_tuple("Box").field(v).finish(),
            Value::Circle(v) => f.debug_tuple("Circle").field(v).finish(),
            Value::LTree(v) => f.debug_tuple("LTree").field(v).finish(),
            Value::LQuery(v) => f.debug_tuple("LQuery").field(v).finish(),
            Value::Cube(v) => f.debug_tuple("Cube").field(v).finish(),
            Value::Int4Range(v) => f.debug_tuple("Int4Range").field(v).finish(),
            Value::Int8Range(v) => f.debug_tuple("Int8Range").field(v).finish(),
            Value::NumRange(v) => f.debug_tuple("NumRange").field(v).finish(),
            Value::TsRange(v) => f.debug_tuple("TsRange").field(v).finish(),
            Value::TsTzRange(v) => f.debug_tuple("TsTzRange").field(v).finish(),
            Value::DateRange(v) => f.debug_tuple("DateRange").field(v).finish(),
            Value::Int4Multirange(v) => f.debug_tuple("Int4Multirange").field(v).finish(),
            Value::Int8Multirange(v) => f.debug_tuple("Int8Multirange").field(v).finish(),
            Value::NumMultirange(v) => f.debug_tuple("NumMultirange").field(v).finish(),
            Value::TsMultirange(v) => f.debug_tuple("TsMultirange").field(v).finish(),
            Value::TsTzMultirange(v) => f.debug_tuple("TsTzMultirange").field(v).finish(),
            Value::DateMultirange(v) => f.debug_tuple("DateMultirange").field(v).finish(),
            Value::Array(v) => f.debug_tuple("Array").field(v).finish(),
            Value::Custom(_) => f.debug_tuple("Custom").field(&"<opaque>").finish(),
        }
    }
}

// Manual PartialEq: Arc<dyn ToSql> isn't PartialEq, so Custom values are never equal.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::BigInt(a), Value::BigInt(b)) => a == b,
            (Value::Double(a), Value::Double(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Uuid(a), Value::Uuid(b)) => a == b,
            (Value::Timestamp(a), Value::Timestamp(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::SmallInt(a), Value::SmallInt(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Numeric(a), Value::Numeric(b)) => a == b,
            (Value::Money(a), Value::Money(b)) => a == b,
            (Value::Xml(a), Value::Xml(b)) => a == b,
            (Value::PgLsn(a), Value::PgLsn(b)) => a == b,
            (Value::Bit(a), Value::Bit(b)) => a == b,
            (Value::Json(a), Value::Json(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Time(a), Value::Time(b)) => a == b,
            (Value::TimeTz(a), Value::TimeTz(b)) => a == b,
            (Value::TimestampNaive(a), Value::TimestampNaive(b)) => a == b,
            (Value::Inet(a), Value::Inet(b)) => a == b,
            (Value::Cidr(a), Value::Cidr(b)) => a == b,
            (Value::MacAddr(a), Value::MacAddr(b)) => a == b,
            (Value::MacAddr8(a), Value::MacAddr8(b)) => a == b,
            (Value::Interval(a), Value::Interval(b)) => a == b,
            (Value::Point(a), Value::Point(b)) => a == b,
            (Value::Line(a), Value::Line(b)) => a == b,
            (Value::LineSegment(a), Value::LineSegment(b)) => a == b,
            (Value::Box(a), Value::Box(b)) => a == b,
            (Value::Circle(a), Value::Circle(b)) => a == b,
            (Value::LTree(a), Value::LTree(b)) => a == b,
            (Value::LQuery(a), Value::LQuery(b)) => a == b,
            (Value::Cube(a), Value::Cube(b)) => a == b,
            (Value::Int4Range(a), Value::Int4Range(b)) => a == b,
            (Value::Int8Range(a), Value::Int8Range(b)) => a == b,
            (Value::NumRange(a), Value::NumRange(b)) => a == b,
            (Value::TsRange(a), Value::TsRange(b)) => a == b,
            (Value::TsTzRange(a), Value::TsTzRange(b)) => a == b,
            (Value::DateRange(a), Value::DateRange(b)) => a == b,
            (Value::Int4Multirange(a), Value::Int4Multirange(b)) => a == b,
            (Value::Int8Multirange(a), Value::Int8Multirange(b)) => a == b,
            (Value::NumMultirange(a), Value::NumMultirange(b)) => a == b,
            (Value::TsMultirange(a), Value::TsMultirange(b)) => a == b,
            (Value::TsTzMultirange(a), Value::TsTzMultirange(b)) => a == b,
            (Value::DateMultirange(a), Value::DateMultirange(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Custom(_), Value::Custom(_)) => false,
            _ => false,
        }
    }
}

// === From impls ===

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

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::SmallInt(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v)
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        Value::Json(v)
    }
}

impl From<IpAddr> for Value {
    fn from(v: IpAddr) -> Self {
        Value::Inet(v)
    }
}

impl From<NaiveDate> for Value {
    fn from(v: NaiveDate) -> Self {
        Value::Date(v)
    }
}

impl From<NaiveTime> for Value {
    fn from(v: NaiveTime) -> Self {
        Value::Time(v)
    }
}

impl From<NaiveDateTime> for Value {
    fn from(v: NaiveDateTime) -> Self {
        Value::TimestampNaive(v)
    }
}

impl From<rust_decimal::Decimal> for Value {
    fn from(v: rust_decimal::Decimal) -> Self {
        Value::Numeric(v)
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

// === ToSql ===

impl driver::ToSql for Value {
    fn oid(&self) -> driver::Oid {
        match self {
            Value::Null => driver::Oid::TEXT,
            Value::Bool(_) => driver::Oid::BOOL,
            Value::Int(_) => driver::Oid::INT4,
            Value::BigInt(_) => driver::Oid::INT8,
            Value::Double(_) => driver::Oid::FLOAT8,
            Value::Text(_) => driver::Oid::TEXT,
            Value::Uuid(_) => driver::Oid::UUID,
            Value::Timestamp(_) => driver::Oid::TIMESTAMPTZ,
            Value::Bytes(_) => driver::Oid::BYTEA,
            Value::SmallInt(_) => driver::Oid::INT2,
            Value::Float(_) => driver::Oid::FLOAT4,
            Value::Numeric(_) => driver::Oid::NUMERIC,
            Value::Money(_) => driver::Oid::MONEY,
            Value::Xml(_) => driver::Oid::XML,
            Value::PgLsn(_) => driver::Oid::PG_LSN,
            Value::Bit(v) => v.oid(),
            Value::Json(_) => driver::Oid::JSONB,
            Value::Date(_) => driver::Oid::DATE,
            Value::Time(_) => driver::Oid::TIME,
            Value::TimeTz(v) => v.oid(),
            Value::TimestampNaive(_) => driver::Oid::TIMESTAMP,
            Value::Inet(_) => driver::Oid::INET,
            Value::Cidr(_) => driver::Oid::CIDR,
            Value::MacAddr(_) => driver::Oid::MACADDR,
            Value::MacAddr8(v) => v.oid(),
            Value::Interval(v) => v.oid(),
            Value::Point(v) => v.oid(),
            Value::Line(v) => v.oid(),
            Value::LineSegment(v) => v.oid(),
            Value::Box(v) => v.oid(),
            Value::Circle(v) => v.oid(),
            Value::LTree(v) => v.oid(),
            Value::LQuery(v) => v.oid(),
            Value::Cube(v) => v.oid(),
            Value::Int4Range(v) => v.oid(),
            Value::Int8Range(v) => v.oid(),
            Value::NumRange(v) => v.oid(),
            Value::TsRange(v) => v.oid(),
            Value::TsTzRange(v) => v.oid(),
            Value::DateRange(v) => v.oid(),
            Value::Int4Multirange(v) => v.oid(),
            Value::Int8Multirange(v) => v.oid(),
            Value::NumMultirange(v) => v.oid(),
            Value::TsMultirange(v) => v.oid(),
            Value::TsTzMultirange(v) => v.oid(),
            Value::DateMultirange(v) => v.oid(),
            Value::Array(elements) => {
                let elem_oid = elements
                    .iter()
                    .find(|v| !matches!(v, Value::Null))
                    .map(|v| v.oid());
                match elem_oid {
                    Some(driver::Oid::BOOL) => driver::Oid::BOOL_ARRAY,
                    Some(driver::Oid::INT2) => driver::Oid::INT2_ARRAY,
                    Some(driver::Oid::INT4) => driver::Oid::INT4_ARRAY,
                    Some(driver::Oid::INT8) => driver::Oid::INT8_ARRAY,
                    Some(driver::Oid::FLOAT4) => driver::Oid::FLOAT4_ARRAY,
                    Some(driver::Oid::FLOAT8) => driver::Oid::FLOAT8_ARRAY,
                    Some(driver::Oid::TEXT | driver::Oid::VARCHAR) => driver::Oid::TEXT_ARRAY,
                    Some(driver::Oid::UUID) => driver::Oid::UUID_ARRAY,
                    Some(driver::Oid::NUMERIC) => driver::Oid::NUMERIC_ARRAY,
                    Some(driver::Oid::INET) => driver::Oid::INET_ARRAY,
                    Some(driver::Oid::INTERVAL) => driver::Oid::INTERVAL_ARRAY,
                    Some(driver::Oid::JSONB) => driver::Oid::JSONB_ARRAY,
                    Some(driver::Oid::TIMESTAMP) => driver::Oid::TIMESTAMP_ARRAY,
                    Some(driver::Oid::TIMESTAMPTZ) => driver::Oid::TIMESTAMPTZ_ARRAY,
                    Some(driver::Oid::DATE) => driver::Oid::DATE_ARRAY,
                    Some(driver::Oid::TIME) => driver::Oid::TIME_ARRAY,
                    Some(driver::Oid::BYTEA) => driver::Oid::BYTEA_ARRAY,
                    Some(driver::Oid::MONEY) => driver::Oid::MONEY_ARRAY,
                    _ => driver::Oid::TEXT_ARRAY,
                }
            }
            Value::Custom(v) => v.oid(),
        }
    }

    fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    fn to_sql(&self, buf: &mut bytes::BytesMut) -> driver::Result<()> {
        use bytes::BufMut;

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
            Value::SmallInt(v) => v.to_sql(buf),
            Value::Float(v) => v.to_sql(buf),
            Value::Numeric(v) => v.to_sql(buf),
            Value::Money(v) => driver::types::money::PgMoney(*v).to_sql(buf),
            Value::Xml(v) => driver::types::xml::PgXml(v.clone()).to_sql(buf),
            Value::PgLsn(v) => driver::types::lsn::PgLsn(*v).to_sql(buf),
            Value::Bit(v) => v.to_sql(buf),
            Value::Json(v) => {
                // JSONB binary format: 1-byte version prefix (0x01) + JSON text
                buf.put_u8(1);
                // serde_json::to_vec on serde_json::Value is infallible
                let json_bytes = serde_json::to_vec(v).expect("serde_json::Value serialization");
                buf.put_slice(&json_bytes);
                Ok(())
            }
            Value::Date(v) => v.to_sql(buf),
            Value::Time(v) => v.to_sql(buf),
            Value::TimeTz(v) => v.to_sql(buf),
            Value::TimestampNaive(v) => v.to_sql(buf),
            Value::Inet(v) => v.to_sql(buf),
            Value::Cidr(v) => {
                let mask = if v.is_ipv4() { 32 } else { 128 };
                driver::types::network::PgCidr {
                    addr: *v,
                    netmask: mask,
                }
                .to_sql(buf)
            }
            Value::MacAddr(v) => driver::types::network::PgMacAddr(*v).to_sql(buf),
            Value::MacAddr8(v) => v.to_sql(buf),
            Value::Interval(v) => v.to_sql(buf),
            Value::Point(v) => v.to_sql(buf),
            Value::Line(v) => v.to_sql(buf),
            Value::LineSegment(v) => v.to_sql(buf),
            Value::Box(v) => v.to_sql(buf),
            Value::Circle(v) => v.to_sql(buf),
            Value::LTree(v) => v.to_sql(buf),
            Value::LQuery(v) => v.to_sql(buf),
            Value::Cube(v) => v.to_sql(buf),
            Value::Int4Range(v) => v.to_sql(buf),
            Value::Int8Range(v) => v.to_sql(buf),
            Value::NumRange(v) => v.to_sql(buf),
            Value::TsRange(v) => v.to_sql(buf),
            Value::TsTzRange(v) => v.to_sql(buf),
            Value::DateRange(v) => v.to_sql(buf),
            Value::Int4Multirange(v) => v.to_sql(buf),
            Value::Int8Multirange(v) => v.to_sql(buf),
            Value::NumMultirange(v) => v.to_sql(buf),
            Value::TsMultirange(v) => v.to_sql(buf),
            Value::TsTzMultirange(v) => v.to_sql(buf),
            Value::DateMultirange(v) => v.to_sql(buf),
            Value::Array(elements) => Self::encode_array(elements, buf),
            Value::Custom(v) => v.to_sql(buf),
        }
    }
}

// === Accessor methods ===

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }
    pub fn is_bigint(&self) -> bool {
        matches!(self, Value::BigInt(_))
    }
    pub fn is_double(&self) -> bool {
        matches!(self, Value::Double(_))
    }
    pub fn is_text(&self) -> bool {
        matches!(self, Value::Text(_))
    }
    pub fn is_uuid(&self) -> bool {
        matches!(self, Value::Uuid(_))
    }
    pub fn is_timestamp(&self) -> bool {
        matches!(self, Value::Timestamp(_))
    }
    pub fn is_bytes(&self) -> bool {
        matches!(self, Value::Bytes(_))
    }
    pub fn is_smallint(&self) -> bool {
        matches!(self, Value::SmallInt(_))
    }
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }
    pub fn is_numeric(&self) -> bool {
        matches!(self, Value::Numeric(_))
    }
    pub fn is_money(&self) -> bool {
        matches!(self, Value::Money(_))
    }
    pub fn is_xml(&self) -> bool {
        matches!(self, Value::Xml(_))
    }
    pub fn is_pglsn(&self) -> bool {
        matches!(self, Value::PgLsn(_))
    }
    pub fn is_bit(&self) -> bool {
        matches!(self, Value::Bit(_))
    }
    pub fn is_json(&self) -> bool {
        matches!(self, Value::Json(_))
    }
    pub fn is_date(&self) -> bool {
        matches!(self, Value::Date(_))
    }
    pub fn is_time(&self) -> bool {
        matches!(self, Value::Time(_))
    }
    pub fn is_timetz(&self) -> bool {
        matches!(self, Value::TimeTz(_))
    }
    pub fn is_timestamp_naive(&self) -> bool {
        matches!(self, Value::TimestampNaive(_))
    }
    pub fn is_inet(&self) -> bool {
        matches!(self, Value::Inet(_))
    }
    pub fn is_cidr(&self) -> bool {
        matches!(self, Value::Cidr(_))
    }
    pub fn is_macaddr(&self) -> bool {
        matches!(self, Value::MacAddr(_))
    }
    pub fn is_macaddr8(&self) -> bool {
        matches!(self, Value::MacAddr8(_))
    }
    pub fn is_interval(&self) -> bool {
        matches!(self, Value::Interval(_))
    }
    pub fn is_point(&self) -> bool {
        matches!(self, Value::Point(_))
    }
    pub fn is_line(&self) -> bool {
        matches!(self, Value::Line(_))
    }
    pub fn is_line_segment(&self) -> bool {
        matches!(self, Value::LineSegment(_))
    }
    pub fn is_box(&self) -> bool {
        matches!(self, Value::Box(_))
    }
    pub fn is_circle(&self) -> bool {
        matches!(self, Value::Circle(_))
    }
    pub fn is_ltree(&self) -> bool {
        matches!(self, Value::LTree(_))
    }
    pub fn is_lquery(&self) -> bool {
        matches!(self, Value::LQuery(_))
    }
    pub fn is_cube(&self) -> bool {
        matches!(self, Value::Cube(_))
    }
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }
    pub fn is_custom(&self) -> bool {
        matches!(self, Value::Custom(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_bigint(&self) -> Option<i64> {
        match self {
            Value::BigInt(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_double(&self) -> Option<f64> {
        match self {
            Value::Double(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Value::Uuid(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_timestamp(&self) -> Option<DateTime<Utc>> {
        match self {
            Value::Timestamp(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_smallint(&self) -> Option<i16> {
        match self {
            Value::SmallInt(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_float(&self) -> Option<f32> {
        match self {
            Value::Float(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_numeric(&self) -> Option<rust_decimal::Decimal> {
        match self {
            Value::Numeric(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_money(&self) -> Option<i64> {
        match self {
            Value::Money(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Value::Json(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_date(&self) -> Option<NaiveDate> {
        match self {
            Value::Date(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_time(&self) -> Option<NaiveTime> {
        match self {
            Value::Time(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_timestamp_naive(&self) -> Option<NaiveDateTime> {
        match self {
            Value::TimestampNaive(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_inet(&self) -> Option<IpAddr> {
        match self {
            Value::Inet(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_interval(&self) -> Option<&driver::types::interval::PgInterval> {
        match self {
            Value::Interval(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_point(&self) -> Option<driver::types::geometric::PgPoint> {
        match self {
            Value::Point(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_timetz(&self) -> Option<driver::types::timetz::PgTimeTz> {
        match self {
            Value::TimeTz(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_macaddr8(&self) -> Option<&driver::types::network::PgMacAddr8> {
        match self {
            Value::MacAddr8(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_ltree(&self) -> Option<&driver::types::ltree::PgLTree> {
        match self {
            Value::LTree(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_lquery(&self) -> Option<&driver::types::ltree::PgLQuery> {
        match self {
            Value::LQuery(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_cube(&self) -> Option<&driver::types::cube::PgCube> {
        match self {
            Value::Cube(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }
}

// === Array helpers ===

impl Value {
    fn encode_array(elements: &[Value], buf: &mut bytes::BytesMut) -> driver::Result<()> {
        use bytes::BufMut;

        if elements.is_empty() {
            return Err(driver::Error::Encode("empty array".into()));
        }

        let element_oid = elements
            .iter()
            .find(|v| !matches!(v, Value::Null))
            .map(|v| v.oid())
            .ok_or_else(|| {
                driver::Error::Encode("all-null array cannot determine element OID".into())
            })?;

        let has_null = elements.iter().any(|v| matches!(v, Value::Null));

        // PG binary array format:
        // 4 bytes: ndim (1 for 1D)
        // 4 bytes: has_null flag
        // 4 bytes: element OID
        // 4 bytes: array length
        // 4 bytes: lower bound (1-based)
        buf.put_i32(1); // ndim
        buf.put_i32(i32::from(has_null));
        buf.put_u32(element_oid.0);
        buf.put_i32(elements.len() as i32);
        buf.put_i32(1); // lower bound

        // For each element: 4 bytes length + data (or -1 for null)
        for elem in elements {
            if matches!(elem, Value::Null) {
                buf.put_i32(-1);
            } else {
                let len_pos = buf.len();
                buf.put_i32(0); // placeholder
                elem.to_sql(buf)?;
                let data_len = (buf.len() - len_pos - 4) as i32;
                buf[len_pos..len_pos + 4].copy_from_slice(&data_len.to_be_bytes());
            }
        }

        Ok(())
    }
}

// === Display ===

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Int(v) => write!(f, "{v}"),
            Value::BigInt(v) => write!(f, "{v}"),
            Value::Double(v) => write!(f, "{v}"),
            Value::Text(v) => write!(f, "'{v}'"),
            Value::Uuid(v) => write!(f, "{v}"),
            Value::Timestamp(v) => write!(f, "{v}"),
            Value::Bytes(v) => {
                write!(f, "\\x")?;
                for b in v {
                    write!(f, "{b:02x}")?;
                }
                Ok(())
            }
            Value::SmallInt(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Numeric(v) => write!(f, "{v}"),
            Value::Money(v) => write!(f, "{v}"),
            Value::Xml(v) => write!(f, "{v}"),
            Value::PgLsn(v) => write!(f, "{:X}/{:X}", v >> 32, v & 0xFFFF_FFFF),
            Value::Bit(v) => write!(f, "{v:?}"),
            Value::Json(v) => write!(f, "{v}"),
            Value::Date(v) => write!(f, "{v}"),
            Value::Time(v) => write!(f, "{v}"),
            Value::TimeTz(v) => write!(f, "{:?}", v),
            Value::TimestampNaive(v) => write!(f, "{v}"),
            Value::Inet(v) => write!(f, "{v}"),
            Value::Cidr(v) => write!(f, "{v}"),
            Value::MacAddr(m) => write!(
                f,
                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                m[0], m[1], m[2], m[3], m[4], m[5]
            ),
            Value::MacAddr8(m) => write!(
                f,
                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                m.0[0], m.0[1], m.0[2], m.0[3], m.0[4], m.0[5], m.0[6], m.0[7]
            ),
            Value::Interval(v) => write!(f, "{v:?}"),
            Value::Point(v) => write!(f, "({},{})", v.x, v.y),
            Value::Line(v) => write!(f, "{{{},{},{}}}", v.a, v.b, v.c),
            Value::LineSegment(v) => {
                write!(
                    f,
                    "[({},{}),({},{})]",
                    v.start.x, v.start.y, v.end.x, v.end.y
                )
            }
            Value::Box(v) => write!(
                f,
                "(({},{}),({},{}))",
                v.upper_right.x, v.upper_right.y, v.lower_left.x, v.lower_left.y
            ),
            Value::Circle(v) => write!(f, "<({},{}),{}>", v.center.x, v.center.y, v.radius),
            Value::LTree(v) => write!(f, "{v}"),
            Value::LQuery(v) => write!(f, "{v}"),
            Value::Cube(v) => write!(f, "{v}"),
            Value::Int4Range(v) => write!(f, "{v:?}"),
            Value::Int8Range(v) => write!(f, "{v:?}"),
            Value::NumRange(v) => write!(f, "{v:?}"),
            Value::TsRange(v) => write!(f, "{v:?}"),
            Value::TsTzRange(v) => write!(f, "{v:?}"),
            Value::DateRange(v) => write!(f, "{v:?}"),
            Value::Int4Multirange(v) => write!(f, "{v:?}"),
            Value::Int8Multirange(v) => write!(f, "{v:?}"),
            Value::NumMultirange(v) => write!(f, "{v:?}"),
            Value::TsMultirange(v) => write!(f, "{v:?}"),
            Value::TsTzMultirange(v) => write!(f, "{v:?}"),
            Value::DateMultirange(v) => write!(f, "{v:?}"),
            Value::Array(elements) => {
                write!(f, "{{")?;
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{e}")?;
                }
                write!(f, "}}")
            }
            Value::Custom(_) => write!(f, "<custom>"),
        }
    }
}
