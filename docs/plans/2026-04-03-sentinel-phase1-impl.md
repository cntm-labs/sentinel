# Sentinel ORM — Phase 1: Foundation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Establish the Rust workspace, core type system, Model trait, and query builder that generates parameterized SQL — all without a live database connection.

**Architecture:** Phase 1 builds `sentinel-core` as a pure library that generates SQL strings + bind parameters. No database driver needed — execution is behind a `Connection` trait. All other crates (`sentinel-macros`, `sentinel-migrate`, `sentinel-cli`) are scaffolded as empty stubs. Query builder uses the builder pattern with type-safe column references.

**Tech Stack:** Rust (stable), tokio (async runtime), thiserror (error derive), chrono (DateTime), uuid (Uuid)

**Design Doc:** `docs/plans/2026-04-03-sentinel-design.md`

**Driver:** `sentinel-driver` (separate repo at `../sentinel-driver`) provides the PG wire protocol implementation. Phase 1 does NOT depend on the driver — we generate SQL + bind params only. The `Connection` trait (Phase 4) will be implemented by sentinel-driver's `Pool`/`Connection` types. Some integration tests may fail until sentinel-driver is available — this is expected.

---

## Task 1: Workspace Setup

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `sentinel-core/Cargo.toml`
- Create: `sentinel-core/src/lib.rs`
- Create: `sentinel-macros/Cargo.toml`
- Create: `sentinel-macros/src/lib.rs`
- Create: `sentinel-migrate/Cargo.toml`
- Create: `sentinel-migrate/src/lib.rs`
- Create: `sentinel-cli/Cargo.toml`
- Create: `sentinel-cli/src/main.rs`

**Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
members = [
    "sentinel-core",
    "sentinel-macros",
    "sentinel-migrate",
    "sentinel-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
repository = "https://github.com/sentinel-orm/sentinel"
rust-version = "1.85"

[workspace.dependencies]
sentinel-core = { path = "sentinel-core" }
sentinel-macros = { path = "sentinel-macros" }
sentinel-migrate = { path = "sentinel-migrate" }
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

**Step 2: Create sentinel-core crate**

`sentinel-core/Cargo.toml`:
```toml
[package]
name = "sentinel-core"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror.workspace = true
chrono.workspace = true
uuid.workspace = true
tokio.workspace = true
```

`sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.
```

**Step 3: Create sentinel-macros crate (stub)**

`sentinel-macros/Cargo.toml`:
```toml
[package]
name = "sentinel-macros"
version.workspace = true
edition.workspace = true

[lib]
proc-macro = true

[dependencies]
```

`sentinel-macros/src/lib.rs`:
```rust
//! Sentinel Macros — derive(Model), derive(Partial), #[reducer].
//! Phase 1: Stub crate, implementation in Phase 2.
```

**Step 4: Create sentinel-migrate crate (stub)**

`sentinel-migrate/Cargo.toml`:
```toml
[package]
name = "sentinel-migrate"
version.workspace = true
edition.workspace = true

[dependencies]
sentinel-core.workspace = true
```

`sentinel-migrate/src/lib.rs`:
```rust
//! Sentinel Migrate — Schema diff and migration generation.
//! Phase 1: Stub crate, implementation in later phase.
```

**Step 5: Create sentinel-cli crate (stub)**

`sentinel-cli/Cargo.toml`:
```toml
[package]
name = "sentinel-cli"
version.workspace = true
edition.workspace = true

[dependencies]
sentinel-core.workspace = true
sentinel-migrate.workspace = true
```

`sentinel-cli/src/main.rs`:
```rust
//! Sentinel CLI — command-line tool.
//! Phase 1: Stub binary, implementation in later phase.

fn main() {
    println!("sentinel-cli: not yet implemented");
}
```

**Step 6: Initialize git and verify**

```bash
cd /Users/mrbt/Desktop/repository/orm/sentinel
git init
cargo check --workspace
cargo fmt --all
```

Expected: All crates compile with zero errors.

**Step 7: Commit**

```bash
git add -A
git commit -m "chore: scaffold workspace with 4 crates"
```

---

## Task 2: Error Types

**Files:**
- Create: `sentinel-core/src/error.rs`
- Modify: `sentinel-core/src/lib.rs`
- Create: `sentinel-core/tests/error_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/error_test.rs`:
```rust
use sentinel_core::Error;

#[test]
fn error_display_column_not_found() {
    let err = Error::ColumnNotFound {
        column: "email".into(),
        table: "users".into(),
    };
    assert_eq!(
        err.to_string(),
        "column 'email' not found in table 'users'"
    );
}

#[test]
fn error_display_query_build() {
    let err = Error::QueryBuild("missing WHERE clause".into());
    assert_eq!(err.to_string(), "query build error: missing WHERE clause");
}

#[test]
fn error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Error>();
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test error_test
```

Expected: FAIL — `Error` type does not exist.

**Step 3: Write implementation**

`sentinel-core/src/error.rs`:
```rust
/// Sentinel error types.
///
/// All errors are `Send + Sync` so they work across async boundaries.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("column '{column}' not found in table '{table}'")]
    ColumnNotFound { column: String, table: String },

    #[error("query build error: {0}")]
    QueryBuild(String),

    #[error("connection error: {0}")]
    Connection(String),

    #[error("transaction error: {0}")]
    Transaction(String),

    #[error("row not found")]
    NotFound,

    #[error("type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },
}

/// Sentinel result type alias.
pub type Result<T> = std::result::Result<T, Error>;
```

Update `sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;

pub use error::{Error, Result};
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test error_test
```

Expected: 3 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/error.rs sentinel-core/src/lib.rs sentinel-core/tests/error_test.rs
git commit -m "feat(core): add Error enum with thiserror derives"
```

---

## Task 3: SQL Value Types

**Files:**
- Create: `sentinel-core/src/types/mod.rs`
- Create: `sentinel-core/src/types/value.rs`
- Modify: `sentinel-core/src/lib.rs`
- Create: `sentinel-core/tests/value_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/value_test.rs`:
```rust
use sentinel_core::types::Value;
use chrono::{TimeZone, Utc};
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
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test value_test
```

Expected: FAIL — `types` module does not exist.

**Step 3: Write implementation**

`sentinel-core/src/types/mod.rs`:
```rust
mod value;

pub use value::Value;
```

`sentinel-core/src/types/value.rs`:
```rust
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

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(inner) => inner.into(),
            None => Value::Null,
        }
    }
}
```

Update `sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod types;

pub use error::{Error, Result};
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test value_test
```

Expected: 11 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/types/ sentinel-core/src/lib.rs sentinel-core/tests/value_test.rs
git commit -m "feat(core): add Value enum with From impls for SQL types"
```

---

## Task 4: Column Definition and Expressions

**Files:**
- Create: `sentinel-core/src/types/column.rs`
- Create: `sentinel-core/src/expr.rs`
- Modify: `sentinel-core/src/types/mod.rs`
- Modify: `sentinel-core/src/lib.rs`
- Create: `sentinel-core/tests/expr_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/expr_test.rs`:
```rust
use sentinel_core::expr::{Expr, Column};
use sentinel_core::types::Value;

#[test]
fn column_eq_generates_expr() {
    let col = Column::new("users", "email");
    let expr = col.eq("alice@example.com");
    assert_eq!(expr.to_sql(1), "\"users\".\"email\" = $1");
    assert_eq!(expr.binds(), vec![Value::Text("alice@example.com".into())]);
}

#[test]
fn column_ne_generates_expr() {
    let col = Column::new("users", "role");
    let expr = col.ne("admin");
    assert_eq!(expr.to_sql(1), "\"users\".\"role\" != $1");
}

#[test]
fn column_gt_lt() {
    let col = Column::new("users", "age");
    assert_eq!(col.gt(25i32).to_sql(1), "\"users\".\"age\" > $1");
    assert_eq!(col.lt(65i32).to_sql(1), "\"users\".\"age\" < $1");
    assert_eq!(col.gte(25i32).to_sql(1), "\"users\".\"age\" >= $1");
    assert_eq!(col.lte(65i32).to_sql(1), "\"users\".\"age\" <= $1");
}

#[test]
fn column_is_null() {
    let col = Column::new("users", "deleted_at");
    let expr = col.is_null();
    assert_eq!(expr.to_sql(1), "\"users\".\"deleted_at\" IS NULL");
    assert!(expr.binds().is_empty());
}

#[test]
fn column_is_not_null() {
    let col = Column::new("users", "email");
    let expr = col.is_not_null();
    assert_eq!(expr.to_sql(1), "\"users\".\"email\" IS NOT NULL");
}

#[test]
fn column_like() {
    let col = Column::new("users", "name");
    let expr = col.like("%alice%");
    assert_eq!(expr.to_sql(1), "\"users\".\"name\" LIKE $1");
    assert_eq!(expr.binds(), vec![Value::Text("%alice%".into())]);
}

#[test]
fn column_in_list() {
    let col = Column::new("users", "status");
    let expr = col.in_list(vec![Value::from("active"), Value::from("pending")]);
    assert_eq!(expr.to_sql(1), "\"users\".\"status\" IN ($1, $2)");
    assert_eq!(expr.binds().len(), 2);
}

#[test]
fn expr_and_combines() {
    let c1 = Column::new("users", "age");
    let c2 = Column::new("users", "active");
    let expr = c1.gt(18i32).and(c2.eq(true));
    assert_eq!(expr.to_sql(1), "(\"users\".\"age\" > $1 AND \"users\".\"active\" = $2)");
}

#[test]
fn expr_or_combines() {
    let c1 = Column::new("users", "role");
    let c2 = Column::new("users", "role");
    let expr = c1.eq("admin").or(c2.eq("moderator"));
    assert_eq!(expr.to_sql(1), "(\"users\".\"role\" = $1 OR \"users\".\"role\" = $2)");
}

#[test]
fn column_desc_asc() {
    let col = Column::new("users", "created_at");
    assert_eq!(col.desc().to_sql_bare(), "\"users\".\"created_at\" DESC");
    assert_eq!(col.asc().to_sql_bare(), "\"users\".\"created_at\" ASC");
}

#[test]
fn bind_index_chains_correctly() {
    let c1 = Column::new("users", "name");
    let c2 = Column::new("users", "email");
    let c3 = Column::new("users", "age");
    let expr = c1.eq("Alice").and(c2.eq("alice@ex.com")).and(c3.gt(18i32));
    let sql = expr.to_sql(1);
    // Should produce $1, $2, $3 in order
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));
    assert!(sql.contains("$3"));
    assert_eq!(expr.binds().len(), 3);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test expr_test
```

Expected: FAIL — `expr` module does not exist.

**Step 3: Write implementation**

`sentinel-core/src/expr.rs`:
```rust
use crate::types::Value;

/// A reference to a table column, used to build type-safe expressions.
#[derive(Debug, Clone)]
pub struct Column {
    pub table: String,
    pub name: String,
}

impl Column {
    pub fn new(table: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
        }
    }

    /// Fully qualified and quoted column reference: `"table"."column"`
    pub fn qualified(&self) -> String {
        format!("\"{}\".\"{}\"", self.table, self.name)
    }

    pub fn eq(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "=",
            value: val.into(),
        }
    }

    pub fn ne(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "!=",
            value: val.into(),
        }
    }

    pub fn gt(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: ">",
            value: val.into(),
        }
    }

    pub fn lt(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "<",
            value: val.into(),
        }
    }

    pub fn gte(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: ">=",
            value: val.into(),
        }
    }

    pub fn lte(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "<=",
            value: val.into(),
        }
    }

    pub fn like(&self, pattern: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "LIKE",
            value: pattern.into(),
        }
    }

    pub fn is_null(&self) -> Expr {
        Expr::IsNull {
            column: self.qualified(),
            negated: false,
        }
    }

    pub fn is_not_null(&self) -> Expr {
        Expr::IsNull {
            column: self.qualified(),
            negated: true,
        }
    }

    pub fn in_list(&self, values: Vec<Value>) -> Expr {
        Expr::InList {
            column: self.qualified(),
            values,
        }
    }

    pub fn desc(&self) -> OrderExpr {
        OrderExpr {
            column: self.qualified(),
            direction: "DESC",
        }
    }

    pub fn asc(&self) -> OrderExpr {
        OrderExpr {
            column: self.qualified(),
            direction: "ASC",
        }
    }
}

/// An ordering expression for ORDER BY clauses.
#[derive(Debug, Clone)]
pub struct OrderExpr {
    column: String,
    direction: &'static str,
}

impl OrderExpr {
    pub fn to_sql_bare(&self) -> String {
        format!("{} {}", self.column, self.direction)
    }
}

/// A filter expression that generates parameterized SQL.
///
/// Bind parameter indices start from a given offset and increment sequentially.
/// This ensures composed expressions produce correct `$1, $2, ...` placeholders.
#[derive(Debug, Clone)]
pub enum Expr {
    Compare {
        column: String,
        op: &'static str,
        value: Value,
    },
    IsNull {
        column: String,
        negated: bool,
    },
    InList {
        column: String,
        values: Vec<Value>,
    },
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Generate SQL with bind parameters starting at the given index.
    pub fn to_sql(&self, start: usize) -> String {
        match self {
            Expr::Compare { column, op, .. } => {
                format!("{column} {op} ${start}")
            }
            Expr::IsNull { column, negated } => {
                if *negated {
                    format!("{column} IS NOT NULL")
                } else {
                    format!("{column} IS NULL")
                }
            }
            Expr::InList { column, values } => {
                let placeholders: Vec<String> = (0..values.len())
                    .map(|i| format!("${}", start + i))
                    .collect();
                format!("{column} IN ({})", placeholders.join(", "))
            }
            Expr::And(left, right) => {
                let left_sql = left.to_sql(start);
                let left_count = left.bind_count();
                let right_sql = right.to_sql(start + left_count);
                format!("({left_sql} AND {right_sql})")
            }
            Expr::Or(left, right) => {
                let left_sql = left.to_sql(start);
                let left_count = left.bind_count();
                let right_sql = right.to_sql(start + left_count);
                format!("({left_sql} OR {right_sql})")
            }
        }
    }

    /// Collect all bind values in order.
    pub fn binds(&self) -> Vec<Value> {
        match self {
            Expr::Compare { value, .. } => vec![value.clone()],
            Expr::IsNull { .. } => vec![],
            Expr::InList { values, .. } => values.clone(),
            Expr::And(left, right) | Expr::Or(left, right) => {
                let mut v = left.binds();
                v.extend(right.binds());
                v
            }
        }
    }

    /// Number of bind parameters this expression contributes.
    pub fn bind_count(&self) -> usize {
        match self {
            Expr::Compare { .. } => 1,
            Expr::IsNull { .. } => 0,
            Expr::InList { values, .. } => values.len(),
            Expr::And(left, right) | Expr::Or(left, right) => {
                left.bind_count() + right.bind_count()
            }
        }
    }

    /// Combine with AND.
    pub fn and(self, other: Expr) -> Expr {
        Expr::And(Box::new(self), Box::new(other))
    }

    /// Combine with OR.
    pub fn or(self, other: Expr) -> Expr {
        Expr::Or(Box::new(self), Box::new(other))
    }
}
```

Update `sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod types;

pub use error::{Error, Result};
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test expr_test
```

Expected: 11 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/expr.rs sentinel-core/src/lib.rs sentinel-core/tests/expr_test.rs
git commit -m "feat(core): add Column and Expr for type-safe filter expressions"
```

---

## Task 5: SELECT Query Builder

**Files:**
- Create: `sentinel-core/src/query/mod.rs`
- Create: `sentinel-core/src/query/select.rs`
- Modify: `sentinel-core/src/lib.rs`
- Create: `sentinel-core/tests/select_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/select_test.rs`:
```rust
use sentinel_core::expr::Column;
use sentinel_core::query::SelectQuery;

#[test]
fn select_all_from_table() {
    let q = SelectQuery::new("users");
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
    assert!(binds.is_empty());
}

#[test]
fn select_specific_columns() {
    let q = SelectQuery::new("users")
        .columns(vec!["id", "email", "name"]);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"email\", \"users\".\"name\" FROM \"users\""
    );
}

#[test]
fn select_with_where() {
    let col = Column::new("users", "email");
    let q = SelectQuery::new("users")
        .where_(col.eq("alice@example.com"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" WHERE \"users\".\"email\" = $1"
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn select_with_multiple_where() {
    let email = Column::new("users", "email");
    let active = Column::new("users", "active");
    let q = SelectQuery::new("users")
        .where_(email.eq("alice@example.com"))
        .where_(active.eq(true));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" WHERE (\"users\".\"email\" = $1 AND \"users\".\"active\" = $2)"
    );
    assert_eq!(binds.len(), 2);
}

#[test]
fn select_with_order_by() {
    let col = Column::new("users", "created_at");
    let q = SelectQuery::new("users")
        .order_by(col.desc());
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" ORDER BY \"users\".\"created_at\" DESC"
    );
}

#[test]
fn select_with_limit_offset() {
    let q = SelectQuery::new("users")
        .limit(20)
        .offset(40);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" LIMIT 20 OFFSET 40"
    );
}

#[test]
fn select_full_query() {
    let email = Column::new("users", "email");
    let created = Column::new("users", "created_at");
    let q = SelectQuery::new("users")
        .columns(vec!["id", "email"])
        .where_(email.like("%@example.com"))
        .order_by(created.desc())
        .limit(10)
        .offset(0);
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"email\" FROM \"users\" \
         WHERE \"users\".\"email\" LIKE $1 \
         ORDER BY \"users\".\"created_at\" DESC \
         LIMIT 10 OFFSET 0"
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn select_for_update() {
    let q = SelectQuery::new("accounts")
        .for_update();
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"accounts\".* FROM \"accounts\" FOR UPDATE"
    );
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test select_test
```

Expected: FAIL — `query` module does not exist.

**Step 3: Write implementation**

`sentinel-core/src/query/mod.rs`:
```rust
mod select;

pub use select::SelectQuery;
```

`sentinel-core/src/query/select.rs`:
```rust
use crate::expr::{Expr, OrderExpr};
use crate::types::Value;

/// Builder for SELECT queries with parameterized bind values.
#[derive(Debug)]
pub struct SelectQuery {
    table: String,
    columns: Option<Vec<String>>,
    wheres: Vec<Expr>,
    order_bys: Vec<OrderExpr>,
    limit: Option<u64>,
    offset: Option<u64>,
    for_update: bool,
}

impl SelectQuery {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: None,
            wheres: Vec::new(),
            order_bys: Vec::new(),
            limit: None,
            offset: None,
            for_update: false,
        }
    }

    pub fn columns(mut self, cols: Vec<&str>) -> Self {
        self.columns = Some(cols.into_iter().map(String::from).collect());
        self
    }

    pub fn where_(mut self, expr: Expr) -> Self {
        self.wheres.push(expr);
        self
    }

    pub fn order_by(mut self, order: OrderExpr) -> Self {
        self.order_bys.push(order);
        self
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: u64) -> Self {
        self.offset = Some(n);
        self
    }

    pub fn for_update(mut self) -> Self {
        self.for_update = true;
        self
    }

    /// Build the SQL string and bind parameters.
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::new();
        let mut binds = Vec::new();

        // SELECT clause
        sql.push_str("SELECT ");
        match &self.columns {
            Some(cols) => {
                let qualified: Vec<String> = cols
                    .iter()
                    .map(|c| format!("\"{}\".\"{c}\"", self.table))
                    .collect();
                sql.push_str(&qualified.join(", "));
            }
            None => {
                sql.push_str(&format!("\"{}\".*", self.table));
            }
        }

        // FROM clause
        sql.push_str(&format!(" FROM \"{}\"", self.table));

        // WHERE clause
        if !self.wheres.is_empty() {
            let combined = self
                .wheres
                .iter()
                .cloned()
                .reduce(|a, b| a.and(b))
                .unwrap();
            let bind_start = binds.len() + 1;
            sql.push_str(&format!(" WHERE {}", combined.to_sql(bind_start)));
            binds.extend(combined.binds());
        }

        // ORDER BY clause
        if !self.order_bys.is_empty() {
            let orders: Vec<String> = self.order_bys.iter().map(|o| o.to_sql_bare()).collect();
            sql.push_str(&format!(" ORDER BY {}", orders.join(", ")));
        }

        // LIMIT / OFFSET
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {offset}"));
        }

        // FOR UPDATE
        if self.for_update {
            sql.push_str(" FOR UPDATE");
        }

        (sql, binds)
    }
}
```

Update `sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod query;
pub mod types;

pub use error::{Error, Result};
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test select_test
```

Expected: 8 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/query/ sentinel-core/src/lib.rs sentinel-core/tests/select_test.rs
git commit -m "feat(core): add SelectQuery builder with WHERE, ORDER BY, LIMIT"
```

---

## Task 6: INSERT Query Builder

**Files:**
- Create: `sentinel-core/src/query/insert.rs`
- Modify: `sentinel-core/src/query/mod.rs`
- Create: `sentinel-core/tests/insert_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/insert_test.rs`:
```rust
use sentinel_core::query::InsertQuery;
use sentinel_core::types::Value;

#[test]
fn insert_single_row() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .column("name", "Alice");
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\", \"name\") VALUES ($1, $2) RETURNING *"
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0], Value::Text("alice@example.com".into()));
    assert_eq!(binds[1], Value::Text("Alice".into()));
}

#[test]
fn insert_with_returning_specific() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .returning(vec!["id", "email"]);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\") VALUES ($1) RETURNING \"id\", \"email\""
    );
}

#[test]
fn insert_with_no_returning() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .no_returning();
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\") VALUES ($1)"
    );
}

#[test]
fn insert_on_conflict_do_nothing() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .on_conflict_do_nothing("email");
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\") VALUES ($1) \
         ON CONFLICT (\"email\") DO NOTHING RETURNING *"
    );
}

#[test]
fn insert_multiple_values() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .column("name", "Alice")
        .column("active", true);
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\", \"name\", \"active\") VALUES ($1, $2, $3) RETURNING *"
    );
    assert_eq!(binds.len(), 3);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test insert_test
```

Expected: FAIL — `InsertQuery` does not exist.

**Step 3: Write implementation**

`sentinel-core/src/query/insert.rs`:
```rust
use crate::types::Value;

/// Controls what INSERT returns.
#[derive(Debug, Clone)]
enum Returning {
    /// RETURNING * (default)
    All,
    /// RETURNING "col1", "col2"
    Columns(Vec<String>),
    /// No RETURNING clause
    None,
}

/// Builder for INSERT queries.
#[derive(Debug)]
pub struct InsertQuery {
    table: String,
    columns: Vec<String>,
    values: Vec<Value>,
    returning: Returning,
    on_conflict: Option<String>,
}

impl InsertQuery {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: Vec::new(),
            values: Vec::new(),
            returning: Returning::All,
            on_conflict: None,
        }
    }

    pub fn column(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.columns.push(name.to_owned());
        self.values.push(value.into());
        self
    }

    pub fn returning(mut self, cols: Vec<&str>) -> Self {
        self.returning = Returning::Columns(cols.into_iter().map(String::from).collect());
        self
    }

    pub fn no_returning(mut self) -> Self {
        self.returning = Returning::None;
        self
    }

    pub fn on_conflict_do_nothing(mut self, conflict_column: &str) -> Self {
        self.on_conflict = Some(conflict_column.to_owned());
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::new();

        // INSERT INTO
        let cols: Vec<String> = self.columns.iter().map(|c| format!("\"{c}\"")).collect();
        let placeholders: Vec<String> = (1..=self.values.len()).map(|i| format!("${i}")).collect();

        sql.push_str(&format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            self.table,
            cols.join(", "),
            placeholders.join(", ")
        ));

        // ON CONFLICT
        if let Some(col) = &self.on_conflict {
            sql.push_str(&format!(" ON CONFLICT (\"{col}\") DO NOTHING"));
        }

        // RETURNING
        match &self.returning {
            Returning::All => sql.push_str(" RETURNING *"),
            Returning::Columns(cols) => {
                let quoted: Vec<String> = cols.iter().map(|c| format!("\"{c}\"")).collect();
                sql.push_str(&format!(" RETURNING {}", quoted.join(", ")));
            }
            Returning::None => {}
        }

        (sql, self.values.clone())
    }
}
```

Update `sentinel-core/src/query/mod.rs`:
```rust
mod insert;
mod select;

pub use insert::InsertQuery;
pub use select::SelectQuery;
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test insert_test
```

Expected: 5 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/query/insert.rs sentinel-core/src/query/mod.rs sentinel-core/tests/insert_test.rs
git commit -m "feat(core): add InsertQuery builder with RETURNING and ON CONFLICT"
```

---

## Task 7: UPDATE Query Builder

**Files:**
- Create: `sentinel-core/src/query/update.rs`
- Modify: `sentinel-core/src/query/mod.rs`
- Create: `sentinel-core/tests/update_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/update_test.rs`:
```rust
use sentinel_core::expr::Column;
use sentinel_core::query::UpdateQuery;
use sentinel_core::types::Value;

#[test]
fn update_single_field() {
    let q = UpdateQuery::new("users")
        .set("name", "Alice Smith")
        .where_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "UPDATE \"users\" SET \"name\" = $1 WHERE \"id\" = $2 RETURNING *"
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0], Value::Text("Alice Smith".into()));
}

#[test]
fn update_multiple_fields() {
    let q = UpdateQuery::new("users")
        .set("name", "Alice Smith")
        .set("active", false)
        .where_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "UPDATE \"users\" SET \"name\" = $1, \"active\" = $2 WHERE \"id\" = $3 RETURNING *"
    );
    assert_eq!(binds.len(), 3);
}

#[test]
fn update_with_where_expr() {
    let col = Column::new("users", "role");
    let q = UpdateQuery::new("users")
        .set("active", false)
        .where_(col.eq("banned"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "UPDATE \"users\" SET \"active\" = $1 WHERE \"users\".\"role\" = $2 RETURNING *"
    );
    assert_eq!(binds.len(), 2);
}

#[test]
fn update_no_returning() {
    let q = UpdateQuery::new("users")
        .set("name", "Alice")
        .where_id(Value::from("id-1"))
        .no_returning();
    let (sql, _) = q.build();
    assert!(sql.ends_with("WHERE \"id\" = $2"));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test update_test
```

Expected: FAIL — `UpdateQuery` does not exist.

**Step 3: Write implementation**

`sentinel-core/src/query/update.rs`:
```rust
use crate::expr::Expr;
use crate::types::Value;

/// Controls what UPDATE returns.
#[derive(Debug, Clone)]
enum Returning {
    All,
    None,
}

/// Builder for UPDATE queries.
#[derive(Debug)]
pub struct UpdateQuery {
    table: String,
    sets: Vec<(String, Value)>,
    where_expr: Option<Expr>,
    returning: Returning,
}

impl UpdateQuery {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            sets: Vec::new(),
            where_expr: None,
            returning: Returning::All,
        }
    }

    pub fn set(mut self, column: &str, value: impl Into<Value>) -> Self {
        self.sets.push((column.to_owned(), value.into()));
        self
    }

    /// Simple WHERE id = $N filter.
    pub fn where_id(mut self, id: Value) -> Self {
        self.where_expr = Some(Expr::Compare {
            column: "\"id\"".to_owned(),
            op: "=",
            value: id,
        });
        self
    }

    /// Custom WHERE expression.
    pub fn where_(mut self, expr: Expr) -> Self {
        self.where_expr = Some(expr);
        self
    }

    pub fn no_returning(mut self) -> Self {
        self.returning = Returning::None;
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::new();
        let mut binds = Vec::new();
        let mut idx = 1usize;

        // UPDATE ... SET
        sql.push_str(&format!("UPDATE \"{}\" SET ", self.table));
        let set_clauses: Vec<String> = self
            .sets
            .iter()
            .map(|(col, val)| {
                let clause = format!("\"{}\" = ${}", col, idx);
                idx += 1;
                binds.push(val.clone());
                clause
            })
            .collect();
        sql.push_str(&set_clauses.join(", "));

        // WHERE
        if let Some(expr) = &self.where_expr {
            sql.push_str(&format!(" WHERE {}", expr.to_sql(idx)));
            binds.extend(expr.binds());
        }

        // RETURNING
        if matches!(self.returning, Returning::All) {
            sql.push_str(" RETURNING *");
        }

        (sql, binds)
    }
}
```

Update `sentinel-core/src/query/mod.rs`:
```rust
mod insert;
mod select;
mod update;

pub use insert::InsertQuery;
pub use select::SelectQuery;
pub use update::UpdateQuery;
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test update_test
```

Expected: 4 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/query/update.rs sentinel-core/src/query/mod.rs sentinel-core/tests/update_test.rs
git commit -m "feat(core): add UpdateQuery builder with SET and WHERE"
```

---

## Task 8: DELETE Query Builder

**Files:**
- Create: `sentinel-core/src/query/delete.rs`
- Modify: `sentinel-core/src/query/mod.rs`
- Create: `sentinel-core/tests/delete_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/delete_test.rs`:
```rust
use sentinel_core::expr::Column;
use sentinel_core::query::DeleteQuery;
use sentinel_core::types::Value;

#[test]
fn delete_by_id() {
    let q = DeleteQuery::new("users").where_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1");
    assert_eq!(binds.len(), 1);
}

#[test]
fn delete_with_where_expr() {
    let col = Column::new("users", "active");
    let q = DeleteQuery::new("users").where_(col.eq(false));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "DELETE FROM \"users\" WHERE \"users\".\"active\" = $1"
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn delete_with_returning() {
    let q = DeleteQuery::new("users")
        .where_id(Value::from("abc-123"))
        .returning();
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "DELETE FROM \"users\" WHERE \"id\" = $1 RETURNING *"
    );
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test delete_test
```

Expected: FAIL — `DeleteQuery` does not exist.

**Step 3: Write implementation**

`sentinel-core/src/query/delete.rs`:
```rust
use crate::expr::Expr;
use crate::types::Value;

/// Builder for DELETE queries.
#[derive(Debug)]
pub struct DeleteQuery {
    table: String,
    where_expr: Option<Expr>,
    returning: bool,
}

impl DeleteQuery {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            where_expr: None,
            returning: false,
        }
    }

    pub fn where_id(mut self, id: Value) -> Self {
        self.where_expr = Some(Expr::Compare {
            column: "\"id\"".to_owned(),
            op: "=",
            value: id,
        });
        self
    }

    pub fn where_(mut self, expr: Expr) -> Self {
        self.where_expr = Some(expr);
        self
    }

    pub fn returning(mut self) -> Self {
        self.returning = true;
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = format!("DELETE FROM \"{}\"", self.table);
        let mut binds = Vec::new();

        if let Some(expr) = &self.where_expr {
            sql.push_str(&format!(" WHERE {}", expr.to_sql(1)));
            binds.extend(expr.binds());
        }

        if self.returning {
            sql.push_str(" RETURNING *");
        }

        (sql, binds)
    }
}
```

Update `sentinel-core/src/query/mod.rs`:
```rust
mod delete;
mod insert;
mod select;
mod update;

pub use delete::DeleteQuery;
pub use insert::InsertQuery;
pub use select::SelectQuery;
pub use update::UpdateQuery;
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test delete_test
```

Expected: 3 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/query/delete.rs sentinel-core/src/query/mod.rs sentinel-core/tests/delete_test.rs
git commit -m "feat(core): add DeleteQuery builder"
```

---

## Task 9: Model Trait

**Files:**
- Create: `sentinel-core/src/model.rs`
- Modify: `sentinel-core/src/lib.rs`
- Create: `sentinel-core/tests/model_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/model_test.rs`:
```rust
use sentinel_core::expr::Column;
use sentinel_core::model::{Model, ModelColumn};
use sentinel_core::types::Value;

/// A manually implemented Model for testing.
/// In Phase 2, `derive(Model)` will generate this.
struct User;

impl Model for User {
    const TABLE: &'static str = "users";
    const PRIMARY_KEY: &'static str = "id";

    fn columns() -> &'static [ModelColumn] {
        &USER_COLUMNS
    }
}

static USER_COLUMNS: [ModelColumn; 4] = [
    ModelColumn {
        name: "id",
        column_type: "uuid",
        nullable: false,
        has_default: true,
    },
    ModelColumn {
        name: "email",
        column_type: "text",
        nullable: false,
        has_default: false,
    },
    ModelColumn {
        name: "name",
        column_type: "text",
        nullable: true,
        has_default: false,
    },
    ModelColumn {
        name: "created_at",
        column_type: "timestamptz",
        nullable: false,
        has_default: true,
    },
];

// Column constants (derive(Model) will generate these)
impl User {
    const ID: Column = Column { table: std::borrow::Cow::Borrowed("users"), name: std::borrow::Cow::Borrowed("id") };
    const EMAIL: Column = Column { table: std::borrow::Cow::Borrowed("users"), name: std::borrow::Cow::Borrowed("email") };
    const NAME: Column = Column { table: std::borrow::Cow::Borrowed("users"), name: std::borrow::Cow::Borrowed("name") };
    const CREATED_AT: Column = Column { table: std::borrow::Cow::Borrowed("users"), name: std::borrow::Cow::Borrowed("created_at") };
}

#[test]
fn model_has_table_name() {
    assert_eq!(User::TABLE, "users");
}

#[test]
fn model_has_primary_key() {
    assert_eq!(User::PRIMARY_KEY, "id");
}

#[test]
fn model_columns_returns_metadata() {
    let cols = User::columns();
    assert_eq!(cols.len(), 4);
    assert_eq!(cols[0].name, "id");
    assert!(!cols[1].nullable);
    assert!(cols[2].nullable); // name is Option<String>
    assert!(cols[3].has_default); // created_at has default
}

#[test]
fn model_column_constants_build_expressions() {
    let expr = User::EMAIL.eq("alice@example.com");
    assert_eq!(expr.to_sql(1), "\"users\".\"email\" = $1");
}

#[test]
fn model_find_builds_select() {
    let q = User::find();
    let (sql, _) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
}

#[test]
fn model_find_by_id_builds_select() {
    let q = User::find_by_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" WHERE \"id\" = $1"
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn model_delete_by_id() {
    let q = User::delete(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1");
    assert_eq!(binds.len(), 1);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test model_test
```

Expected: FAIL — `model` module does not exist.

**Step 3: Write implementation**

This step requires refactoring `Column` to use `Cow<'static, str>` so it can be used in `const` context. Update `sentinel-core/src/expr.rs` — change `Column` fields:

```rust
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct Column {
    pub table: Cow<'static, str>,
    pub name: Cow<'static, str>,
}

impl Column {
    pub fn new(table: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            table: Cow::Owned(table.into()),
            name: Cow::Owned(name.into()),
        }
    }

    pub fn qualified(&self) -> String {
        format!("\"{}\".\"{}\"", self.table, self.name)
    }

    // ... rest of methods unchanged
}
```

`sentinel-core/src/model.rs`:
```rust
use crate::query::{DeleteQuery, SelectQuery};
use crate::types::Value;
use crate::expr::Expr;

/// Metadata for a single column in a model.
pub struct ModelColumn {
    pub name: &'static str,
    pub column_type: &'static str,
    pub nullable: bool,
    pub has_default: bool,
}

/// Core trait that all Sentinel models implement.
///
/// In Phase 2, `#[derive(Model)]` generates this automatically.
/// For Phase 1, models implement this manually for testing.
pub trait Model {
    /// The PostgreSQL table name.
    const TABLE: &'static str;

    /// The primary key column name (default: "id").
    const PRIMARY_KEY: &'static str;

    /// Returns column metadata for this model.
    fn columns() -> &'static [ModelColumn];

    /// Start a SELECT query for this model's table.
    fn find() -> SelectQuery {
        SelectQuery::new(Self::TABLE)
    }

    /// SELECT ... WHERE id = $1
    fn find_by_id(id: Value) -> SelectQuery {
        SelectQuery::new(Self::TABLE).where_(Expr::Compare {
            column: format!("\"{}\"", Self::PRIMARY_KEY),
            op: "=",
            value: id,
        })
    }

    /// DELETE ... WHERE id = $1
    fn delete(id: Value) -> DeleteQuery {
        DeleteQuery::new(Self::TABLE).where_id(id)
    }
}
```

Update `sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod model;
pub mod query;
pub mod types;

pub use error::{Error, Result};
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test model_test
```

Expected: 7 tests PASS. Also run all existing tests to verify refactor didn't break anything:

```bash
cargo test -p sentinel-core
```

Expected: All tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/model.rs sentinel-core/src/expr.rs sentinel-core/src/lib.rs sentinel-core/tests/model_test.rs
git commit -m "feat(core): add Model trait with find/find_by_id/delete, refactor Column to Cow"
```

---

## Task 10: Dynamic QueryBuilder (Layer 4)

**Files:**
- Create: `sentinel-core/src/query/dynamic.rs`
- Modify: `sentinel-core/src/query/mod.rs`
- Create: `sentinel-core/tests/dynamic_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/dynamic_test.rs`:
```rust
use sentinel_core::query::QueryBuilder;
use sentinel_core::types::Value;

#[test]
fn dynamic_select() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.column("email");
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"id\", \"email\" FROM \"users\"");
    assert!(binds.is_empty());
}

#[test]
fn dynamic_select_with_where() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.where_eq("active", true);
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"id\" FROM \"users\" WHERE \"active\" = $1");
    assert_eq!(binds.len(), 1);
}

#[test]
fn dynamic_select_multiple_where() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.where_eq("active", true);
    q.where_eq("role", "admin");
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"id\" FROM \"users\" WHERE \"active\" = $1 AND \"role\" = $2"
    );
    assert_eq!(binds.len(), 2);
}

#[test]
fn dynamic_select_order_limit() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.order_by_desc("created_at");
    q.limit(10);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"id\" FROM \"users\" ORDER BY \"created_at\" DESC LIMIT 10"
    );
}

#[test]
fn always_parameterized() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.where_eq("name", "Robert'); DROP TABLE users;--");
    let (sql, binds) = q.build();
    // Value is in binds, NOT in SQL string
    assert_eq!(sql, "SELECT \"id\" FROM \"users\" WHERE \"name\" = $1");
    assert_eq!(binds[0], Value::Text("Robert'); DROP TABLE users;--".into()));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test dynamic_test
```

Expected: FAIL — `QueryBuilder` does not exist.

**Step 3: Write implementation**

`sentinel-core/src/query/dynamic.rs`:
```rust
use crate::types::Value;

/// Dynamic query builder (Layer 4) — for queries built at runtime.
///
/// Always parameterized — values are never interpolated into SQL strings.
/// This is the escape hatch for queries that can't be expressed with the
/// typed builders, while still preventing SQL injection.
#[derive(Debug)]
pub struct QueryBuilder {
    table: String,
    columns: Vec<String>,
    wheres: Vec<(String, Value)>,
    order_bys: Vec<String>,
    limit: Option<u64>,
}

impl QueryBuilder {
    /// Start building a SELECT query.
    pub fn select_from(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: Vec::new(),
            wheres: Vec::new(),
            order_bys: Vec::new(),
            limit: None,
        }
    }

    /// Add a column to the SELECT clause.
    pub fn column(&mut self, name: &str) -> &mut Self {
        self.columns.push(name.to_owned());
        self
    }

    /// Add a WHERE column = $N condition.
    pub fn where_eq(&mut self, column: &str, value: impl Into<Value>) -> &mut Self {
        self.wheres.push((column.to_owned(), value.into()));
        self
    }

    /// Add ORDER BY column DESC.
    pub fn order_by_desc(&mut self, column: &str) -> &mut Self {
        self.order_bys.push(format!("\"{}\" DESC", column));
        self
    }

    /// Add ORDER BY column ASC.
    pub fn order_by_asc(&mut self, column: &str) -> &mut Self {
        self.order_bys.push(format!("\"{}\" ASC", column));
        self
    }

    /// Set LIMIT.
    pub fn limit(&mut self, n: u64) -> &mut Self {
        self.limit = Some(n);
        self
    }

    /// Build the final SQL and bind parameters.
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::new();
        let mut binds = Vec::new();

        // SELECT
        let cols = if self.columns.is_empty() {
            "*".to_owned()
        } else {
            self.columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ")
        };
        sql.push_str(&format!("SELECT {} FROM \"{}\"", cols, self.table));

        // WHERE
        if !self.wheres.is_empty() {
            let clauses: Vec<String> = self
                .wheres
                .iter()
                .enumerate()
                .map(|(i, (col, val))| {
                    binds.push(val.clone());
                    format!("\"{}\" = ${}", col, i + 1)
                })
                .collect();
            sql.push_str(&format!(" WHERE {}", clauses.join(" AND ")));
        }

        // ORDER BY
        if !self.order_bys.is_empty() {
            sql.push_str(&format!(" ORDER BY {}", self.order_bys.join(", ")));
        }

        // LIMIT
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        (sql, binds)
    }
}
```

Update `sentinel-core/src/query/mod.rs`:
```rust
mod delete;
mod dynamic;
mod insert;
mod select;
mod update;

pub use delete::DeleteQuery;
pub use dynamic::QueryBuilder;
pub use insert::InsertQuery;
pub use select::SelectQuery;
pub use update::UpdateQuery;
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test dynamic_test
```

Expected: 5 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/query/dynamic.rs sentinel-core/src/query/mod.rs sentinel-core/tests/dynamic_test.rs
git commit -m "feat(core): add dynamic QueryBuilder (Layer 4) — always parameterized"
```

---

## Task 11: Prelude Module

**Files:**
- Create: `sentinel-core/src/prelude.rs`
- Modify: `sentinel-core/src/lib.rs`
- Create: `sentinel-core/tests/prelude_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/prelude_test.rs`:
```rust
/// Verify that the prelude exposes all commonly used types.
use sentinel_core::prelude::*;

#[test]
fn prelude_exposes_core_types() {
    // This test passes if it compiles — verifies the prelude re-exports work
    let _col = Column::new("t", "c");
    let _val = Value::from(42i64);
    let _q = SelectQuery::new("t");
    let _q = InsertQuery::new("t");
    let _q = UpdateQuery::new("t");
    let _q = DeleteQuery::new("t");
    let _q = QueryBuilder::select_from("t");
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test prelude_test
```

Expected: FAIL — `prelude` module does not exist.

**Step 3: Write implementation**

`sentinel-core/src/prelude.rs`:
```rust
//! Common imports for Sentinel users.
//!
//! ```rust
//! use sentinel_core::prelude::*;
//! ```

pub use crate::error::{Error, Result};
pub use crate::expr::{Column, Expr, OrderExpr};
pub use crate::model::{Model, ModelColumn};
pub use crate::query::{DeleteQuery, InsertQuery, QueryBuilder, SelectQuery, UpdateQuery};
pub use crate::types::Value;
```

Update `sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod model;
pub mod prelude;
pub mod query;
pub mod types;

pub use error::{Error, Result};
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test prelude_test
```

Expected: 1 test PASS.

**Step 5: Run full test suite and clippy**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: All tests pass, no clippy warnings, formatting clean.

**Step 6: Commit**

```bash
git add sentinel-core/src/prelude.rs sentinel-core/src/lib.rs sentinel-core/tests/prelude_test.rs
git commit -m "feat(core): add prelude module for convenient imports"
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Workspace setup (4 crates) | cargo check |
| 2 | Error types | 3 tests |
| 3 | Value enum + From impls | 11 tests |
| 4 | Column + Expr (filters) | 11 tests |
| 5 | SelectQuery builder | 8 tests |
| 6 | InsertQuery builder | 5 tests |
| 7 | UpdateQuery builder | 4 tests |
| 8 | DeleteQuery builder | 3 tests |
| 9 | Model trait | 7 tests |
| 10 | Dynamic QueryBuilder | 5 tests |
| 11 | Prelude module | 1 test (compile check) |

**Total: 11 tasks, ~58 tests, 11 commits**

After Phase 1, Sentinel can generate parameterized SQL for all CRUD operations with type-safe expressions. No database connection needed — that comes with sentinel-driver integration.

### Future Phases (separate plans)

- **Phase 2:** `sentinel-macros` — `derive(Model)`, `derive(Partial)`, `#[reducer]`
- **Phase 3:** Type-state relations — `User<Bare>` vs `User<WithPosts>`, include/batch_load
- **Phase 4:** Connection trait + sentinel-driver integration
- **Phase 5:** Transaction system with deadlock prevention
- **Phase 6:** `sentinel-migrate` — schema diff, SQL generation
- **Phase 7:** `sentinel-cli` — CLI commands
