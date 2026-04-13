use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use driver::ToSql;
use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

/// A dynamically-typed SQL value used in query parameters.
///
/// Covers all PostgreSQL types supported by sentinel-driver v0.1.1.
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
    TimestampNaive(NaiveDateTime),

    // === Network ===
    Inet(IpAddr),
    Cidr(IpAddr),
    MacAddr([u8; 6]),

    // === Interval ===
    Interval(driver::types::interval::PgInterval),

    // === Geometric ===
    Point(driver::types::geometric::PgPoint),
    Line(driver::types::geometric::PgLine),
    LineSegment(driver::types::geometric::PgLSeg),
    Box(driver::types::geometric::PgBox),
    Circle(driver::types::geometric::PgCircle),

    // === Ranges ===
    Int4Range(driver::types::range::PgRange<i32>),
    Int8Range(driver::types::range::PgRange<i64>),
    NumRange(driver::types::range::PgRange<rust_decimal::Decimal>),
    TsRange(driver::types::range::PgRange<NaiveDateTime>),
    TsTzRange(driver::types::range::PgRange<DateTime<Utc>>),
    DateRange(driver::types::range::PgRange<NaiveDate>),

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
            Value::TimestampNaive(v) => f.debug_tuple("TimestampNaive").field(v).finish(),
            Value::Inet(v) => f.debug_tuple("Inet").field(v).finish(),
            Value::Cidr(v) => f.debug_tuple("Cidr").field(v).finish(),
            Value::MacAddr(v) => f.debug_tuple("MacAddr").field(v).finish(),
            Value::Interval(v) => f.debug_tuple("Interval").field(v).finish(),
            Value::Point(v) => f.debug_tuple("Point").field(v).finish(),
            Value::Line(v) => f.debug_tuple("Line").field(v).finish(),
            Value::LineSegment(v) => f.debug_tuple("LineSegment").field(v).finish(),
            Value::Box(v) => f.debug_tuple("Box").field(v).finish(),
            Value::Circle(v) => f.debug_tuple("Circle").field(v).finish(),
            Value::Int4Range(v) => f.debug_tuple("Int4Range").field(v).finish(),
            Value::Int8Range(v) => f.debug_tuple("Int8Range").field(v).finish(),
            Value::NumRange(v) => f.debug_tuple("NumRange").field(v).finish(),
            Value::TsRange(v) => f.debug_tuple("TsRange").field(v).finish(),
            Value::TsTzRange(v) => f.debug_tuple("TsTzRange").field(v).finish(),
            Value::DateRange(v) => f.debug_tuple("DateRange").field(v).finish(),
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
            (Value::TimestampNaive(a), Value::TimestampNaive(b)) => a == b,
            (Value::Inet(a), Value::Inet(b)) => a == b,
            (Value::Cidr(a), Value::Cidr(b)) => a == b,
            (Value::MacAddr(a), Value::MacAddr(b)) => a == b,
            (Value::Interval(a), Value::Interval(b)) => a == b,
            (Value::Point(a), Value::Point(b)) => a == b,
            (Value::Line(a), Value::Line(b)) => a == b,
            (Value::LineSegment(a), Value::LineSegment(b)) => a == b,
            (Value::Box(a), Value::Box(b)) => a == b,
            (Value::Circle(a), Value::Circle(b)) => a == b,
            (Value::Int4Range(a), Value::Int4Range(b)) => a == b,
            (Value::Int8Range(a), Value::Int8Range(b)) => a == b,
            (Value::NumRange(a), Value::NumRange(b)) => a == b,
            (Value::TsRange(a), Value::TsRange(b)) => a == b,
            (Value::TsTzRange(a), Value::TsTzRange(b)) => a == b,
            (Value::DateRange(a), Value::DateRange(b)) => a == b,
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
            Value::TimestampNaive(_) => driver::Oid::TIMESTAMP,
            Value::Inet(_) => driver::Oid::INET,
            Value::Cidr(_) => driver::Oid::CIDR,
            Value::MacAddr(_) => driver::Oid::MACADDR,
            Value::Interval(v) => v.oid(),
            Value::Point(v) => v.oid(),
            Value::Line(v) => v.oid(),
            Value::LineSegment(v) => v.oid(),
            Value::Box(v) => v.oid(),
            Value::Circle(v) => v.oid(),
            Value::Int4Range(v) => v.oid(),
            Value::Int8Range(v) => v.oid(),
            Value::NumRange(v) => v.oid(),
            Value::TsRange(v) => v.oid(),
            Value::TsTzRange(v) => v.oid(),
            Value::DateRange(v) => v.oid(),
            Value::Array(elements) => Self::array_oid(elements).unwrap_or(driver::Oid::TEXT_ARRAY),
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
                let json_bytes =
                    serde_json::to_vec(v).map_err(|e| driver::Error::Encode(e.to_string()))?;
                buf.put_slice(&json_bytes);
                Ok(())
            }
            Value::Date(v) => v.to_sql(buf),
            Value::Time(v) => v.to_sql(buf),
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
            Value::Interval(v) => v.to_sql(buf),
            Value::Point(v) => v.to_sql(buf),
            Value::Line(v) => v.to_sql(buf),
            Value::LineSegment(v) => v.to_sql(buf),
            Value::Box(v) => v.to_sql(buf),
            Value::Circle(v) => v.to_sql(buf),
            Value::Int4Range(v) => v.to_sql(buf),
            Value::Int8Range(v) => v.to_sql(buf),
            Value::NumRange(v) => v.to_sql(buf),
            Value::TsRange(v) => v.to_sql(buf),
            Value::TsTzRange(v) => v.to_sql(buf),
            Value::DateRange(v) => v.to_sql(buf),
            Value::Array(elements) => Self::encode_array(elements, buf),
            Value::Custom(v) => v.to_sql(buf),
        }
    }
}

// === Array helpers ===

impl Value {
    fn array_oid(elements: &[Value]) -> driver::Result<driver::Oid> {
        let element_oid = elements
            .iter()
            .find(|v| !matches!(v, Value::Null))
            .map(|v| v.oid())
            .ok_or_else(|| {
                driver::Error::Encode("empty or all-null array cannot determine element OID".into())
            })?;

        Ok(match element_oid {
            driver::Oid::BOOL => driver::Oid::BOOL_ARRAY,
            driver::Oid::INT2 => driver::Oid::INT2_ARRAY,
            driver::Oid::INT4 => driver::Oid::INT4_ARRAY,
            driver::Oid::INT8 => driver::Oid::INT8_ARRAY,
            driver::Oid::FLOAT4 => driver::Oid::FLOAT4_ARRAY,
            driver::Oid::FLOAT8 => driver::Oid::FLOAT8_ARRAY,
            driver::Oid::TEXT | driver::Oid::VARCHAR => driver::Oid::TEXT_ARRAY,
            driver::Oid::UUID => driver::Oid::UUID_ARRAY,
            driver::Oid::NUMERIC => driver::Oid::NUMERIC_ARRAY,
            driver::Oid::INET => driver::Oid::INET_ARRAY,
            driver::Oid::INTERVAL => driver::Oid::INTERVAL_ARRAY,
            _ => {
                return Err(driver::Error::Encode(format!(
                    "unsupported array element OID: {:?}",
                    element_oid
                )));
            }
        })
    }

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
            Value::TimestampNaive(v) => write!(f, "{v}"),
            Value::Inet(v) => write!(f, "{v}"),
            Value::Cidr(v) => write!(f, "{v}"),
            Value::MacAddr(m) => write!(
                f,
                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                m[0], m[1], m[2], m[3], m[4], m[5]
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
            Value::Int4Range(v) => write!(f, "{v:?}"),
            Value::Int8Range(v) => write!(f, "{v:?}"),
            Value::NumRange(v) => write!(f, "{v:?}"),
            Value::TsRange(v) => write!(f, "{v:?}"),
            Value::TsTzRange(v) => write!(f, "{v:?}"),
            Value::DateRange(v) => write!(f, "{v:?}"),
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
