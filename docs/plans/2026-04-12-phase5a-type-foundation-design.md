# Phase 5A: Type Foundation — Design Document

**Date:** 2026-04-12
**Status:** Approved
**Depends on:** Phase 4 (merged), sentinel-driver v0.1.1
**Enables:** Phase 5B (type-state relations)

## Goal

Expand Sentinel's type system from 9 Value variants to full PostgreSQL type coverage (66 OIDs from sentinel-driver v0.1.1), add driver re-exports for custom types, and ensure bidirectional encode/decode for all types.

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Type coverage scope | Full 66 OIDs | ORM must be complete tooling, not partial |
| Numeric representation | `String` (no `rust_decimal` dep) | Minimize dependency tree; user converts via driver FromSql |
| Custom PG enums/composites | Re-export driver derives | Driver already has full support; no duplication |
| Pool callbacks | Re-export driver API directly | No unnecessary abstraction |
| Portal/Cursor at ORM level | Not exposed | `fetch_stream()` covers streaming; portal is low-level |
| Observability | Re-export driver API directly | Performance work deferred until real benchmarks exist |

## Value Enum Expansion

### Existing (9 variants)
- `Null`, `Bool`, `Int(i32)`, `BigInt(i64)`, `Double(f64)`, `Text(String)`, `Uuid(Uuid)`, `Timestamp(DateTime<Utc>)`, `Bytes(Vec<u8>)`

### New Scalars (10)
- `SmallInt(i16)` — INT2
- `Float(f32)` — FLOAT4
- `Char(i8)` — CHAR (internal PG type)
- `Oid(u32)` — OID
- `Numeric(String)` — NUMERIC, string representation
- `Money(i64)` — MONEY, cents as i64
- `Xml(String)` — XML
- `PgLsn(u64)` — PG_LSN, WAL position
- `Bit(Vec<u8>)` — BIT/VARBIT, raw bits
- `Json(serde_json::Value)` — JSON/JSONB

### Temporal (3)
- `Date(NaiveDate)` — DATE
- `Time(NaiveTime)` — TIME
- `TimestampNaive(NaiveDateTime)` — TIMESTAMP without timezone

### Network (3)
- `Inet(IpAddr)` — INET
- `Cidr(IpAddr)` — CIDR
- `MacAddr([u8; 6])` — MACADDR

### Interval (1)
- `Interval { months: i32, days: i32, microseconds: i64 }`

### Geometric (7)
- `Point { x: f64, y: f64 }`
- `Line { a: f64, b: f64, c: f64 }`
- `LineSegment { x1: f64, y1: f64, x2: f64, y2: f64 }`
- `Box { x1: f64, y1: f64, x2: f64, y2: f64 }`
- `Path(Vec<(f64, f64)>)`
- `Polygon(Vec<(f64, f64)>)`
- `Circle { x: f64, y: f64, radius: f64 }`

### Ranges (6)
- `Int4Range(Option<i32>, Option<i32>)`
- `Int8Range(Option<i64>, Option<i64>)`
- `NumRange(Option<String>, Option<String>)`
- `TsRange(Option<NaiveDateTime>, Option<NaiveDateTime>)`
- `TsTzRange(Option<DateTime<Utc>>, Option<DateTime<Utc>>)`
- `DateRange(Option<NaiveDate>, Option<NaiveDate>)`

### Collections & Escape Hatch (2)
- `Array(Vec<Value>)` — homogeneous PG arrays
- `Custom(Box<dyn ToSql + Send + Sync>)` — user-defined PG enums, composite types

**Total: 9 existing + 33 new = 42 variants**

## ToSql/FromSql Implementation

### ToSql (Value → PG wire)
- Each variant maps to correct OID for binary encoding
- `Value::Custom(boxed)` delegates to inner `ToSql` impl
- `Value::Array(vec)` infers element OID from first non-null element; errors on mixed types

### FromSql (PG wire → Value)
- Uses OID from `Row::columns()` to dispatch to correct variant
- Unknown OID → `Value::Bytes(raw)` fallback (no error)
- JSON and JSONB decode to same `Value::Json` variant

### Convenience APIs
- `From<T> for Value` — every Rust type that maps directly to a variant
- `is_*()` / `as_*()` — accessor methods for every variant

## Driver Re-exports

Added to `sntl/src/lib.rs`:
- **Derives:** `FromSql`, `ToSql`, `FromRow`
- **Types:** `Oid`, `PgBit`, `RowStream`, `Portal`
- **Pool:** `Pool`, `PoolConfig`, `PooledConnection`
- **Observability:** `ObservabilityConfig`, `QueryMetrics`
- **Connection:** `Connection`, `Config`, `SslMode`

## Custom Type Usage Pattern

```rust
// PG Enum — use driver derive directly
#[derive(sentinel::ToSql, sentinel::FromSql)]
#[sentinel(rename_all = "snake_case")]
enum Status { Active, Inactive, Suspended }

// Composite — use driver derive directly
#[derive(sentinel::ToSql, sentinel::FromSql)]
#[sentinel(type_name = "address")]
struct Address { street: String, city: String, zip: String }

// Use with Value::Custom in queries
InsertQuery::new("users")
    .value("status", Value::Custom(Box::new(Status::Active)))
    .value("address", Value::Custom(Box::new(addr)));
```

## Testing Strategy

### Unit Tests (no DB required)
- Value → SQL roundtrip for every new variant
- `From<T>` impls for every Rust type
- `is_*()` / `as_*()` accessors
- `Value::Array` mixed type detection → error
- `Value::Custom` compile-time trait bound verification

### Integration Tests (PostgreSQL required)
Roundtrip tests per type group in `pg_value_roundtrip_test.rs`:
- Scalars: smallint, float, numeric, money, xml, pg_lsn, bit, json/jsonb
- Temporal: date, time, timestamp naive
- Network: inet, cidr, macaddr
- Interval
- Geometric: point, line, lseg, box, path, polygon, circle
- Ranges: int4range, int8range, numrange, tsrange, tstzrange, daterange
- Arrays: int[], text[], uuid[], nested
- Custom types: PG enum + composite type
- Null roundtrip for all nullable types

Schema additions in `tests/integration/setup.sql`.

## New Dependencies
- `serde_json` — JSON/JSONB support (likely already transitive)
- No new external dependencies (chrono already present)

## Not In Scope
- ORM-level pool wrapper
- ORM-level observability hooks
- Portal/cursor ORM API
- ORM-native `#[derive(PgEnum)]` macros
- `rust_decimal` dependency for Numeric
- Migration system integration

## Phase 5B Preview

Phase 5B (separate design session) will build type-state relation generics on top of this foundation:
1. `#[derive(Model)]` generates per-relation generic parameters
2. Compile-time relation access guard (`.posts()` errors if `Posts = Unloaded`)
3. `.Include()` chain transitions type state
4. `.Load()` / `.BatchLoad()` execution methods
5. Integration tests with diverse column types (enabled by 5A)
