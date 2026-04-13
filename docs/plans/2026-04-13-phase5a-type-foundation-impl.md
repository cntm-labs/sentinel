# Phase 5A: Type Foundation — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expand Sentinel's Value enum from 9 to ~35 variants covering all PostgreSQL types supported by sentinel-driver v0.1.1, add driver re-exports, and validate with roundtrip tests.

**Architecture:** Value enum wraps driver's structured types (PgInterval, PgPoint, etc.) directly rather than reinventing encoding. JSON/JSONB is implemented at ORM level since driver lacks serde_json. Custom user types use `Value::Custom(Box<dyn ToSql>)` escape hatch.

**Tech Stack:** Rust, sentinel-driver v0.1.1, serde_json (new dep), chrono (existing), bytes (existing)

---

### Task 1: Add serde_json dependency

**Files:**
- Modify: `sntl/Cargo.toml`
- Modify: `Cargo.toml` (workspace)

**Step 1: Add serde_json to workspace deps**

In `Cargo.toml` (workspace root), add to `[workspace.dependencies]`:

```toml
serde_json = "1"
serde = { version = "1", features = ["derive"] }
```

**Step 2: Add serde_json to sntl deps**

In `sntl/Cargo.toml`, add under `[dependencies]`:

```toml
serde_json.workspace = true
serde.workspace = true
```

**Step 3: Verify it compiles**

Run: `cargo check --workspace`
Expected: PASS (no code changes yet, just dep addition)

**Step 4: Commit**

```bash
git add Cargo.toml sntl/Cargo.toml Cargo.lock
git commit -m "chore: add serde_json and serde dependencies for JSON/JSONB support"
```

---

### Task 2: Expand Value enum — new scalar variants

**Files:**
- Modify: `sntl/src/core/types/value.rs`
- Test: `sntl/tests/value_test.rs`

**Step 1: Write failing tests for new scalar From<T> impls**

Add to `sntl/tests/value_test.rs`:

```rust
use std::net::{IpAddr, Ipv4Addr};

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
    let dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 13).unwrap()
        .and_hms_opt(14, 30, 0).unwrap();
    let v: Value = dt.into();
    assert!(matches!(v, Value::TimestampNaive(inner) if inner == dt));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test value_test`
Expected: FAIL — variants and From impls don't exist yet

**Step 3: Add new variants and From impls to Value enum**

Replace the entire `sntl/src/core/types/value.rs` with expanded version. Add these new variants to the enum:

```rust
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use std::fmt;
use std::net::IpAddr;
use uuid::Uuid;

/// A dynamically-typed SQL value used in query parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    // === Existing ===
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
    Char(i8),
    Oid(u32),
    Numeric(String),
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
    Custom(std::sync::Arc<dyn driver::ToSql + Send + Sync>),
}
```

**IMPORTANT:** Check whether driver types (PgInterval, PgPoint, etc.) are publicly accessible via `driver::types::*` module path. If they're re-exported at crate root, use that path instead. Grep the driver's `lib.rs` for re-exports.

**NOTE on ranges:** The driver's `PgRange<T>` requires `range_oid` and `element_oid` fields. If adding `rust_decimal` as dependency is undesirable for NumRange, use `PgRange<String>` or skip NumRange. Check if sentinel-driver's `with-rust-decimal` feature is enabled in sntl's Cargo.toml — if not, skip `Decimal` and use `String` for NumRange.

**NOTE on Custom:** Use `Arc` instead of `Box` so Value remains `Clone`. The `PartialEq` impl will need a manual implementation since `dyn ToSql` isn't `PartialEq` — custom values are never equal.

Add all `From<T>` impls:

```rust
impl From<i16> for Value {
    fn from(v: i16) -> Self { Value::SmallInt(v) }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self { Value::Float(v) }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self { Value::Json(v) }
}

impl From<IpAddr> for Value {
    fn from(v: IpAddr) -> Self { Value::Inet(v) }
}

impl From<NaiveDate> for Value {
    fn from(v: NaiveDate) -> Self { Value::Date(v) }
}

impl From<NaiveTime> for Value {
    fn from(v: NaiveTime) -> Self { Value::Time(v) }
}

impl From<NaiveDateTime> for Value {
    fn from(v: NaiveDateTime) -> Self { Value::TimestampNaive(v) }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package sntl --test value_test`
Expected: PASS

**Step 5: Commit**

```bash
git add sntl/src/core/types/value.rs sntl/tests/value_test.rs
git commit -m "feat: expand Value enum with scalar, temporal, network variants"
```

---

### Task 3: Expand Value enum — geometric, interval, range, array, custom variants

**Files:**
- Modify: `sntl/src/core/types/value.rs` (continuing from Task 2)
- Test: `sntl/tests/value_test.rs`

**Step 1: Write failing tests for complex types**

Add to `sntl/tests/value_test.rs`:

```rust
#[test]
fn value_interval() {
    let v = Value::Interval(driver::types::interval::PgInterval {
        months: 1, days: 2, microseconds: 3_000_000,
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test value_test`
Expected: FAIL if these variants aren't added yet (they should be from Task 2)

**Step 3: Verify all variants compile and fix any issues**

If Task 2 added all variants, this step is about fixing compilation. Likely issues:
- `PartialEq` derivation won't work with `Arc<dyn ToSql>` — need manual impl
- Driver type paths may need adjusting
- Range types may need `rust_decimal` feature flag or alternative

Implement manual `PartialEq`:
```rust
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            // ... all variants ...
            (Value::Custom(_), Value::Custom(_)) => false, // custom values never equal
            _ => false,
        }
    }
}
```

**Step 4: Run tests**

Run: `cargo test --package sntl --test value_test`
Expected: PASS

**Step 5: Commit**

```bash
git add sntl/src/core/types/value.rs sntl/tests/value_test.rs
git commit -m "feat: add geometric, interval, range, array, custom Value variants"
```

---

### Task 4: ToSql implementation for all new variants

**Files:**
- Modify: `sntl/src/core/types/value.rs`
- Test: `sntl/tests/value_tosql_test.rs`

**Step 1: Write failing tests for new variant OIDs and encoding**

Add to `sntl/tests/value_tosql_test.rs`:

```rust
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
    let v = Value::Interval(PgInterval { months: 1, days: 2, microseconds: 3_000_000 });
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test value_tosql_test`
Expected: FAIL — new match arms missing in `oid()` and `to_sql()`

**Step 3: Implement ToSql for all new variants**

Expand the `impl driver::ToSql for Value` block. Key patterns:

```rust
impl driver::ToSql for Value {
    fn oid(&self) -> driver::Oid {
        match self {
            // existing...
            Value::SmallInt(_) => driver::Oid::INT2,
            Value::Float(_) => driver::Oid::FLOAT4,
            Value::Char(_) => driver::Oid::CHAR,
            Value::Oid(_) => driver::Oid::OID,
            Value::Numeric(_) => driver::Oid::NUMERIC,
            Value::Money(_) => driver::Oid::MONEY,
            Value::Xml(_) => driver::Oid::XML,
            Value::PgLsn(_) => driver::Oid::PG_LSN,
            Value::Bit(_) => driver::Oid::VARBIT,
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
            Value::Array(_) => driver::Oid::TEXT_ARRAY, // fallback; real impl needs element OID
            Value::Custom(v) => v.oid(),
        }
    }

    fn to_sql(&self, buf: &mut bytes::BytesMut) -> driver::Result<()> {
        match self {
            // existing...
            Value::SmallInt(v) => v.to_sql(buf),
            Value::Float(v) => v.to_sql(buf),
            Value::Char(v) => (*v as i8).to_sql(buf), // verify driver has i8 ToSql
            Value::Oid(v) => buf.put_u32(*v); Ok(()),
            Value::Numeric(v) => v.as_str().to_sql(buf), // text format for numeric-as-string
            Value::Money(v) => {
                use driver::types::money::PgMoney;
                PgMoney(*v).to_sql(buf)
            }
            Value::Xml(v) => {
                use driver::types::xml::PgXml;
                PgXml(v.clone()).to_sql(buf)
            }
            Value::PgLsn(v) => {
                use driver::types::lsn::PgLsn;
                PgLsn(*v).to_sql(buf)
            }
            Value::Bit(v) => v.to_sql(buf),
            Value::Json(v) => {
                // JSONB binary: version prefix byte 1, then JSON text
                buf.put_u8(1);
                let json_bytes = serde_json::to_vec(v)
                    .map_err(|e| driver::Error::Encode(e.to_string()))?;
                buf.put_slice(&json_bytes);
                Ok(())
            }
            Value::Date(v) => v.to_sql(buf),
            Value::Time(v) => v.to_sql(buf),
            Value::TimestampNaive(v) => v.to_sql(buf),
            Value::Inet(v) => v.to_sql(buf), // driver has IpAddr ToSql
            Value::Cidr(v) => {
                use driver::types::network::PgCidr;
                // Default netmask: /32 for v4, /128 for v6
                let mask = if v.is_ipv4() { 32 } else { 128 };
                PgCidr { addr: *v, netmask: mask }.to_sql(buf)
            }
            Value::MacAddr(v) => {
                use driver::types::network::PgMacAddr;
                PgMacAddr(*v).to_sql(buf)
            }
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
            Value::Array(_) => todo!("Array encoding — Task 5"),
            Value::Custom(v) => v.to_sql(buf),
        }
    }
}
```

**IMPORTANT:** The Numeric-as-String approach means we send it as TEXT OID, but PG expects NUMERIC OID for NUMERIC columns. There are two options:
1. Use text protocol for Numeric (send as TEXT, PG auto-casts)
2. Enable `with-rust-decimal` feature on sentinel-driver and use `Decimal` directly

Decision: Enable `with-rust-decimal` feature. It's a small dep and gives correct binary encoding. Update `sntl/Cargo.toml`:
```toml
driver = { package = "sentinel-driver", version = "0.1.1", features = ["with-rust-decimal"] }
```
Then change `Numeric(String)` to `Numeric(rust_decimal::Decimal)` in Value enum.

If we keep String, then use TEXT OID with `::numeric` cast in SQL — but this leaks into query builders. **Prefer Decimal.**

**Step 4: Run tests to verify they pass**

Run: `cargo test --package sntl --test value_tosql_test`
Expected: PASS

**Step 5: Run full test suite**

Run: `cargo test --workspace`
Expected: PASS — existing tests still work

**Step 6: Commit**

```bash
git add sntl/src/core/types/value.rs sntl/tests/value_tosql_test.rs sntl/Cargo.toml
git commit -m "feat: implement ToSql for all new Value variants"
```

---

### Task 5: Array encoding/decoding in Value

**Files:**
- Modify: `sntl/src/core/types/value.rs`
- Test: `sntl/tests/value_tosql_test.rs`

**Step 1: Write failing test**

```rust
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
#[should_panic(expected = "empty array")]
fn value_empty_array_errors() {
    let v = Value::Array(vec![]);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test value_tosql_test`
Expected: FAIL

**Step 3: Implement Array OID inference and encoding**

```rust
impl Value {
    /// Determine the array OID based on element type.
    fn array_oid(elements: &[Value]) -> driver::Result<driver::Oid> {
        let first = elements.first()
            .ok_or_else(|| driver::Error::Encode("empty array cannot determine element OID".into()))?;

        // Skip nulls to find element type
        let element_oid = if first.is_null() {
            elements.iter()
                .find(|v| !v.is_null())
                .map(|v| v.oid())
                .ok_or_else(|| driver::Error::Encode("all-null array cannot determine element OID".into()))?
        } else {
            first.oid()
        };

        // Map element OID to array OID
        Ok(match element_oid {
            driver::Oid::BOOL => driver::Oid::BOOL_ARRAY,
            driver::Oid::INT2 => driver::Oid::INT2_ARRAY,
            driver::Oid::INT4 => driver::Oid::INT4_ARRAY,
            driver::Oid::INT8 => driver::Oid::INT8_ARRAY,
            driver::Oid::FLOAT4 => driver::Oid::FLOAT4_ARRAY,
            driver::Oid::FLOAT8 => driver::Oid::FLOAT8_ARRAY,
            driver::Oid::TEXT | driver::Oid::VARCHAR => driver::Oid::TEXT_ARRAY,
            driver::Oid::UUID => driver::Oid::UUID_ARRAY,
            _ => return Err(driver::Error::Encode(
                format!("unsupported array element OID: {:?}", element_oid)
            )),
        })
    }
}
```

For encoding, use PG binary array format:
- 4 bytes: ndim (1 for 1D)
- 4 bytes: has_null flag
- 4 bytes: element OID
- 4 bytes: array length
- 4 bytes: lower bound (1)
- For each element: 4 bytes length + data (or -1 for null)

**Step 4: Run tests**

Run: `cargo test --package sntl --test value_tosql_test`
Expected: PASS

**Step 5: Commit**

```bash
git add sntl/src/core/types/value.rs sntl/tests/value_tosql_test.rs
git commit -m "feat: implement Array encoding with OID inference for Value"
```

---

### Task 6: Accessor methods (is_*, as_*)

**Files:**
- Modify: `sntl/src/core/types/value.rs`
- Test: `sntl/tests/value_test.rs`

**Step 1: Write failing tests**

Add to `sntl/tests/value_test.rs`:

```rust
#[test]
fn value_is_methods() {
    assert!(Value::SmallInt(1).is_smallint());
    assert!(Value::Float(1.0).is_float());
    assert!(Value::Json(serde_json::json!(null)).is_json());
    assert!(Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).is_date());
    assert!(!Value::Int(1).is_smallint());
}

#[test]
fn value_as_methods() {
    assert_eq!(Value::SmallInt(42).as_smallint(), Some(42));
    assert_eq!(Value::Int(1).as_smallint(), None);
    assert_eq!(Value::Float(1.5).as_float(), Some(1.5));
    assert!(Value::Json(serde_json::json!({"a": 1})).as_json().is_some());
}
```

**Step 2: Run to verify failure**

Run: `cargo test --package sntl --test value_test`
Expected: FAIL

**Step 3: Implement is_* and as_* for every variant**

```rust
impl Value {
    pub fn is_null(&self) -> bool { matches!(self, Value::Null) }
    pub fn is_bool(&self) -> bool { matches!(self, Value::Bool(_)) }
    pub fn is_int(&self) -> bool { matches!(self, Value::Int(_)) }
    pub fn is_smallint(&self) -> bool { matches!(self, Value::SmallInt(_)) }
    pub fn is_bigint(&self) -> bool { matches!(self, Value::BigInt(_)) }
    pub fn is_float(&self) -> bool { matches!(self, Value::Float(_)) }
    pub fn is_double(&self) -> bool { matches!(self, Value::Double(_)) }
    pub fn is_text(&self) -> bool { matches!(self, Value::Text(_)) }
    pub fn is_uuid(&self) -> bool { matches!(self, Value::Uuid(_)) }
    pub fn is_timestamp(&self) -> bool { matches!(self, Value::Timestamp(_)) }
    pub fn is_bytes(&self) -> bool { matches!(self, Value::Bytes(_)) }
    pub fn is_json(&self) -> bool { matches!(self, Value::Json(_)) }
    pub fn is_date(&self) -> bool { matches!(self, Value::Date(_)) }
    pub fn is_time(&self) -> bool { matches!(self, Value::Time(_)) }
    pub fn is_inet(&self) -> bool { matches!(self, Value::Inet(_)) }
    pub fn is_interval(&self) -> bool { matches!(self, Value::Interval(_)) }
    pub fn is_array(&self) -> bool { matches!(self, Value::Array(_)) }
    // ... all variants

    pub fn as_bool(&self) -> Option<bool> { match self { Value::Bool(v) => Some(*v), _ => None } }
    pub fn as_int(&self) -> Option<i32> { match self { Value::Int(v) => Some(*v), _ => None } }
    pub fn as_smallint(&self) -> Option<i16> { match self { Value::SmallInt(v) => Some(*v), _ => None } }
    pub fn as_bigint(&self) -> Option<i64> { match self { Value::BigInt(v) => Some(*v), _ => None } }
    pub fn as_float(&self) -> Option<f32> { match self { Value::Float(v) => Some(*v), _ => None } }
    pub fn as_double(&self) -> Option<f64> { match self { Value::Double(v) => Some(*v), _ => None } }
    pub fn as_text(&self) -> Option<&str> { match self { Value::Text(v) => Some(v), _ => None } }
    pub fn as_uuid(&self) -> Option<uuid::Uuid> { match self { Value::Uuid(v) => Some(*v), _ => None } }
    pub fn as_json(&self) -> Option<&serde_json::Value> { match self { Value::Json(v) => Some(v), _ => None } }
    pub fn as_date(&self) -> Option<NaiveDate> { match self { Value::Date(v) => Some(*v), _ => None } }
    pub fn as_time(&self) -> Option<NaiveTime> { match self { Value::Time(v) => Some(*v), _ => None } }
    pub fn as_inet(&self) -> Option<IpAddr> { match self { Value::Inet(v) => Some(*v), _ => None } }
    // ... all variants with appropriate return types
}
```

**Step 4: Run tests**

Run: `cargo test --package sntl --test value_test`
Expected: PASS

**Step 5: Commit**

```bash
git add sntl/src/core/types/value.rs sntl/tests/value_test.rs
git commit -m "feat: add is_*/as_* accessor methods for all Value variants"
```

---

### Task 7: Driver re-exports

**Files:**
- Modify: `sntl/src/lib.rs`
- Modify: `sntl/src/core/prelude.rs`
- Test: `sntl/tests/prelude_test.rs`

**Step 1: Write failing test**

Add to `sntl/tests/prelude_test.rs`:

```rust
#[test]
fn prelude_exposes_driver_types() {
    // Verify driver types are accessible through sntl
    let _: sntl::driver::types::interval::PgInterval;
    let _: sntl::driver::types::geometric::PgPoint;
    let _: sntl::driver::types::network::PgMacAddr;
}

#[test]
fn reexports_derive_macros() {
    // Just verify the re-export paths compile
    fn _assert_tosql<T: sntl::driver::ToSql>() {}
    fn _assert_fromsql<T: sntl::driver::FromSql>() {}
}
```

**Step 2: Run to verify failure**

Run: `cargo test --package sntl --test prelude_test`
Expected: May PASS if driver types are already public; may FAIL if paths changed

**Step 3: Add re-exports to lib.rs**

In `sntl/src/lib.rs`, add:

```rust
// Re-export driver derive macros for custom PG types
pub use driver::{FromSql, ToSql, FromRow};

// Re-export key driver types for direct use
pub use driver::{Oid, RowStream, Portal};
pub use driver::{Pool, PooledConnection, Config, SslMode};
pub use driver::{ObservabilityConfig, QueryMetrics};
```

In `sntl/src/core/prelude.rs`, add:

```rust
// Driver type re-exports for custom types
pub use driver::{FromSql, ToSql};
```

**Step 4: Run tests**

Run: `cargo test --package sntl --test prelude_test`
Expected: PASS

**Step 5: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS

**Step 6: Commit**

```bash
git add sntl/src/lib.rs sntl/src/core/prelude.rs sntl/tests/prelude_test.rs
git commit -m "feat: re-export driver derives and key types through sntl"
```

---

### Task 8: Integration test schema expansion

**Files:**
- Modify: `tests/integration/setup.sql`
- Modify: `sntl/tests/pg_helpers/mod.rs`

**Step 1: Expand type_roundtrip table**

Replace the `type_roundtrip` table in `tests/integration/setup.sql`:

```sql
DROP TABLE IF EXISTS type_roundtrip CASCADE;

CREATE TABLE type_roundtrip (
    id              SERIAL PRIMARY KEY,
    -- existing
    bool_col        BOOLEAN,
    int_col         INTEGER,
    bigint_col      BIGINT,
    double_col      DOUBLE PRECISION,
    text_col        TEXT,
    uuid_col        UUID,
    timestamptz_col TIMESTAMPTZ,
    bytea_col       BYTEA,
    -- new scalars
    smallint_col    SMALLINT,
    float_col       REAL,
    json_col        JSON,
    jsonb_col       JSONB,
    numeric_col     NUMERIC(20,6),
    money_col       MONEY,
    xml_col         XML,
    bit_col         BIT VARYING(64),
    -- temporal
    date_col        DATE,
    time_col        TIME,
    timestamp_col   TIMESTAMP,
    -- network
    inet_col        INET,
    cidr_col        CIDR,
    macaddr_col     MACADDR,
    -- interval
    interval_col    INTERVAL,
    -- geometric
    point_col       POINT,
    line_col        LINE,
    lseg_col        LSEG,
    box_col         BOX,
    circle_col      CIRCLE,
    -- ranges
    int4range_col   INT4RANGE,
    int8range_col   INT8RANGE,
    numrange_col    NUMRANGE,
    tsrange_col     TSRANGE,
    tstzrange_col   TSTZRANGE,
    daterange_col   DATERANGE,
    -- arrays
    int_array_col   INTEGER[],
    text_array_col  TEXT[]
);
```

**Step 2: Verify schema loads**

Run (if DB available): `psql postgres://sentinel:sentinel_test@localhost:5432/sentinel_test -f tests/integration/setup.sql`
Expected: No errors

**Step 3: Commit**

```bash
git add tests/integration/setup.sql sntl/tests/pg_helpers/mod.rs
git commit -m "feat: expand integration test schema for all PG types"
```

---

### Task 9: Integration roundtrip tests — scalars + temporal + network

**Files:**
- Modify: `sntl/tests/pg_value_roundtrip_test.rs`

**Step 1: Add roundtrip tests for new scalar types**

```rust
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
async fn roundtrip_jsonb() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    let j = serde_json::json!({"key": "value", "num": 42});
    let row = roundtrip_one(&mut conn, "jsonb_col", Value::Json(j.clone())).await;
    // Read back as String, parse to verify
    let val: String = row.get_by_name("jsonb_col");
    let parsed: serde_json::Value = serde_json::from_str(&val).unwrap();
    assert_eq!(parsed, j);
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
async fn roundtrip_interval() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    use sntl::driver::types::interval::PgInterval;
    let iv = PgInterval { months: 1, days: 15, microseconds: 3_600_000_000 };
    let row = roundtrip_one(&mut conn, "interval_col", Value::Interval(iv)).await;
    let val: PgInterval = row.get_by_name("interval_col");
    assert_eq!(val, iv);
}
```

**NOTE:** The `row.get_by_name()` method may not exist in driver — check if it's `row.get(col_index)` or `row.get::<T>(name)`. Adjust accordingly based on driver API. The existing tests use `row.get(index)` with numeric column indices.

**Step 2: Run integration tests**

Run: `DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test cargo test --package sntl --test pg_value_roundtrip_test`
Expected: PASS (or skip if no DB)

**Step 3: Commit**

```bash
git add sntl/tests/pg_value_roundtrip_test.rs
git commit -m "test: add integration roundtrip tests for scalar, temporal, network types"
```

---

### Task 10: Integration roundtrip tests — geometric, ranges, arrays

**Files:**
- Modify: `sntl/tests/pg_value_roundtrip_test.rs`

**Step 1: Add roundtrip tests for complex types**

```rust
#[tokio::test]
async fn roundtrip_point() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    use sntl::driver::types::geometric::PgPoint;
    let pt = PgPoint { x: 1.5, y: 2.5 };
    let row = roundtrip_one(&mut conn, "point_col", Value::Point(pt)).await;
    let val: PgPoint = row.get(/* column index for point_col */);
    assert_eq!(val, pt);
}

#[tokio::test]
async fn roundtrip_circle() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    use sntl::driver::types::geometric::{PgCircle, PgPoint};
    let c = PgCircle { center: PgPoint { x: 0.0, y: 0.0 }, radius: 5.0 };
    let row = roundtrip_one(&mut conn, "circle_col", Value::Circle(c)).await;
    let val: PgCircle = row.get(/* column index */);
    assert_eq!(val, c);
}

#[tokio::test]
async fn roundtrip_int4range() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    use sntl::driver::types::range::{PgRange, RangeBound};
    let r = PgRange {
        lower: RangeBound::Inclusive(1),
        upper: RangeBound::Exclusive(10),
        is_empty: false,
        range_oid: sntl::driver::Oid::INT4RANGE,
        element_oid: sntl::driver::Oid::INT4,
    };
    let row = roundtrip_one(&mut conn, "int4range_col", Value::Int4Range(r.clone())).await;
    // Verify by reading raw or checking bounds
}

#[tokio::test]
async fn roundtrip_int_array() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let row = roundtrip_one(&mut conn, "int_array_col", arr).await;
    let val: Vec<i32> = row.get(/* column index */);
    assert_eq!(val, vec![1, 2, 3]);
}
```

**Step 2: Run integration tests**

Run: `DATABASE_URL=... cargo test --package sntl --test pg_value_roundtrip_test`
Expected: PASS

**Step 3: Commit**

```bash
git add sntl/tests/pg_value_roundtrip_test.rs
git commit -m "test: add integration roundtrip tests for geometric, range, array types"
```

---

### Task 11: Clippy, fmt, full test suite, PR

**Files:**
- All modified files

**Step 1: Run formatter**

Run: `cargo fmt --all`

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Fix any warnings.

**Step 3: Run full test suite**

Run: `cargo test --workspace`
Expected: All PASS

**Step 4: Run integration tests (if DB available)**

Run: `DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test cargo test --workspace`

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: clippy fixes and formatting"
```

**Step 6: Create PR**

```bash
gh pr create --title "feat: Phase 5A — full PostgreSQL type coverage" \
  --body "## Summary
- Expand Value enum from 9 to ~35 variants covering all PG types
- Add ToSql/FromSql for scalars, temporal, network, geometric, ranges, arrays
- JSON/JSONB support via serde_json
- Custom type escape hatch via Value::Custom(Arc<dyn ToSql>)
- Re-export driver derives (FromSql, ToSql, FromRow)
- Integration roundtrip tests for all types

Closes #<issue_number>

## Test plan
- [ ] cargo test --workspace passes
- [ ] Integration roundtrip tests pass with live PG
- [ ] cargo clippy clean"
```

---

## Open Issues to File at cntm-labs/sentinel-driver

After Phase 5A, file these issues:
1. **PATH geometric type** — PgPath struct + ToSql/FromSql (needed for Value::Path)
2. **POLYGON geometric type** — PgPolygon struct + ToSql/FromSql (needed for Value::Polygon)
3. **HSTORE OID constant** — Add Oid::HSTORE to oid.rs
4. **JSON/JSONB ToSql/FromSql** — Add serde_json support in driver (optional feature)
