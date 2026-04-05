# Sentinel ORM — Phase 3: Connection + Driver Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate sentinel-core with sentinel-driver so that `derive(Model)` generates executable async methods that query PostgreSQL via the driver.

**Architecture:** Direct dependency — sentinel-core depends on sentinel-driver (git dependency from `cntm-labs/sentinel-driver`). `Value` enum gets `ToSql` impl to bridge query builders to the driver. `derive(Model)` generates `from_row()` for deserialization and async execution methods (`find_all`, `find_one`, `create`, `delete_by_id`). Query builders gain `.fetch_all(conn)` / `.execute(conn)` methods.

**Tech Stack:** Rust (stable), sentinel-driver v0.1.0, tokio, bytes

**Design Doc:** `docs/plans/2026-04-05-sentinel-phase3-design.md`

---

## Task 1: Add sentinel-driver Dependency + Re-exports

**Files:**
- Modify: `Cargo.toml`
- Modify: `sentinel-core/Cargo.toml`
- Modify: `sentinel-core/src/lib.rs`
- Modify: `sentinel-core/src/prelude.rs`

**Step 1: Add sentinel-driver to workspace dependencies**

Add to `[workspace.dependencies]` in root `Cargo.toml`:
```toml
sentinel-driver = { git = "https://github.com/cntm-labs/sentinel-driver.git", tag = "sentinel-driver-v0.1.0" }
bytes = "1"
```

**Step 2: Add sentinel-driver to sentinel-core dependencies**

In `sentinel-core/Cargo.toml`, add to `[dependencies]`:
```toml
sentinel-driver.workspace = true
bytes.workspace = true
```

**Step 3: Re-export driver types from sentinel-core**

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

// Re-export derive macros
pub use sentinel_macros::Model;
pub use sentinel_macros::Partial;

// Re-export driver types for user convenience
pub use sentinel_driver::{Connection, Pool, Config, SslMode};
pub use sentinel_driver::{IsolationLevel, TransactionConfig, CancelToken};
pub use sentinel_driver::Row;
```

**Step 4: Update prelude with driver re-exports**

Update `sentinel-core/src/prelude.rs`:
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

// Re-export derive macros
pub use sentinel_macros::Model as DeriveModel;
pub use sentinel_macros::Partial as DerivePartial;

// Re-export driver types
pub use sentinel_driver::{Connection, Pool, Config};
```

**Step 5: Verify workspace compiles**

```bash
cargo check --workspace
```

Expected: All crates compile with zero errors.

**Step 6: Commit**

```bash
git add Cargo.toml sentinel-core/Cargo.toml sentinel-core/src/lib.rs sentinel-core/src/prelude.rs
git commit -m "chore: add sentinel-driver dependency and re-export driver types"
```

---

## Task 2: Implement ToSql for Value

**Files:**
- Modify: `sentinel-core/src/types/value.rs`
- Create: `sentinel-core/tests/value_tosql_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/value_tosql_test.rs`:
```rust
use sentinel_core::types::Value;
use sentinel_driver::types::ToSql;
use sentinel_driver::Oid;

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
    let v = Value::Double(3.14);
    assert_eq!(v.oid(), Oid::FLOAT8);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &3.14f64.to_be_bytes());
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
fn value_null_to_sql() {
    let v = Value::Null;
    assert_eq!(v.oid(), Oid::TEXT); // default OID for NULL
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(buf.is_empty()); // NULL writes nothing; caller handles -1 length
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test value_tosql_test
```

Expected: FAIL — `ToSql` not implemented for `Value`.

**Step 3: Implement ToSql for Value**

Add to the end of `sentinel-core/src/types/value.rs`:
```rust
use bytes::BytesMut;

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

    fn to_sql(&self, buf: &mut BytesMut) -> sentinel_driver::Result<()> {
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
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test value_tosql_test
```

Expected: 8 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/types/value.rs sentinel-core/tests/value_tosql_test.rs
git commit -m "feat(core): implement ToSql for Value — bridge to sentinel-driver"
```

---

## Task 3: Add #[must_use] to All Query Builders

**Files:**
- Modify: `sentinel-core/src/query/select.rs`
- Modify: `sentinel-core/src/query/insert.rs`
- Modify: `sentinel-core/src/query/delete.rs`
- Modify: `sentinel-core/src/query/update.rs`
- Modify: `sentinel-core/src/query/dynamic.rs`

**Step 1: Add #[must_use] to each query builder struct**

In each file, add the attribute before the struct definition:

`sentinel-core/src/query/select.rs`:
```rust
#[must_use = "query does nothing until .build() or .fetch_all() is called"]
#[derive(Debug)]
pub struct SelectQuery {
```

`sentinel-core/src/query/insert.rs`:
```rust
#[must_use = "query does nothing until .build() or .execute() is called"]
#[derive(Debug)]
pub struct InsertQuery {
```

`sentinel-core/src/query/delete.rs`:
```rust
#[must_use = "query does nothing until .build() or .execute() is called"]
#[derive(Debug)]
pub struct DeleteQuery {
```

`sentinel-core/src/query/update.rs`:
```rust
#[must_use = "query does nothing until .build() or .execute() is called"]
#[derive(Debug)]
pub struct UpdateQuery {
```

`sentinel-core/src/query/dynamic.rs`:
```rust
#[must_use = "query does nothing until .build() is called"]
#[derive(Debug)]
pub struct QueryBuilder {
```

**Step 2: Verify workspace compiles and tests pass**

```bash
cargo test --workspace
```

Expected: All tests PASS. No regressions.

**Step 3: Commit**

```bash
git add sentinel-core/src/query/
git commit -m "feat(core): add #[must_use] to all query builders"
```

---

## Task 4: Add Error Bridge — Driver Error to Sentinel Error

**Files:**
- Modify: `sentinel-core/src/error.rs`
- Create: `sentinel-core/tests/error_bridge_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/error_bridge_test.rs`:
```rust
use sentinel_core::error::Error;

#[test]
fn driver_error_converts_to_sentinel_error() {
    let driver_err = sentinel_driver::Error::Protocol("test protocol error".into());
    let sentinel_err: Error = driver_err.into();
    assert!(matches!(sentinel_err, Error::Driver(_)));
    assert!(sentinel_err.to_string().contains("test protocol error"));
}

#[test]
fn not_found_error_from_driver() {
    let driver_err = sentinel_driver::Error::Protocol("query returned no rows".into());
    let sentinel_err: Error = driver_err.into();
    assert!(matches!(sentinel_err, Error::Driver(_)));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test error_bridge_test
```

Expected: FAIL — `Error::Driver` variant doesn't exist, no `From<sentinel_driver::Error>` impl.

**Step 3: Add Driver variant and From impl**

Update `sentinel-core/src/error.rs`:
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

    #[error("driver error: {0}")]
    Driver(#[from] sentinel_driver::Error),
}

/// Sentinel result type alias.
pub type Result<T> = std::result::Result<T, Error>;
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test error_bridge_test
```

Expected: 2 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/error.rs sentinel-core/tests/error_bridge_test.rs
git commit -m "feat(core): add Error::Driver variant for sentinel-driver error bridge"
```

---

## Task 5: Add Fetch Methods to SelectQuery

**Files:**
- Modify: `sentinel-core/src/query/select.rs`
- Create: `sentinel-core/tests/fetch_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/fetch_test.rs`:
```rust
use sentinel_core::query::SelectQuery;

// Test that build still works after adding fetch methods
#[test]
fn select_query_build_still_works() {
    let q = SelectQuery::new("users");
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
    assert!(binds.is_empty());
}

// Compile-time test: verify the methods exist with correct signatures
#[allow(dead_code)]
async fn fetch_api_compiles(conn: &mut sentinel_driver::Connection) {
    let q = SelectQuery::new("users");
    let _rows: Vec<sentinel_driver::Row> = q.fetch_all(conn).await.unwrap();

    let q2 = SelectQuery::new("users");
    let _row: sentinel_driver::Row = q2.fetch_one(conn).await.unwrap();

    let q3 = SelectQuery::new("users");
    let _row: Option<sentinel_driver::Row> = q3.fetch_optional(conn).await.unwrap();
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test fetch_test
```

Expected: FAIL — `fetch_all`, `fetch_one`, `fetch_optional` methods don't exist.

**Step 3: Add fetch methods to SelectQuery**

Add to the end of `impl SelectQuery` in `sentinel-core/src/query/select.rs`:
```rust
    /// Execute this query and fetch all rows.
    pub async fn fetch_all(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<Vec<sentinel_driver::Row>> {
        let (sql, binds) = self.build();
        let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> =
            binds.iter().map(|v| v as &(dyn sentinel_driver::ToSql + Sync)).collect();
        Ok(conn.query(&sql, &params).await?)
    }

    /// Execute this query and fetch exactly one row.
    ///
    /// Returns `Error::NotFound` if no rows are returned.
    pub async fn fetch_one(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<sentinel_driver::Row> {
        let (sql, binds) = self.build();
        let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> =
            binds.iter().map(|v| v as &(dyn sentinel_driver::ToSql + Sync)).collect();
        conn.query_one(&sql, &params).await.map_err(Into::into)
    }

    /// Execute this query and fetch an optional row.
    pub async fn fetch_optional(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<Option<sentinel_driver::Row>> {
        let (sql, binds) = self.build();
        let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> =
            binds.iter().map(|v| v as &(dyn sentinel_driver::ToSql + Sync)).collect();
        Ok(conn.query_opt(&sql, &params).await?)
    }
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test fetch_test
```

Expected: 1 test PASS (the build test; the async fn is compile-only).

**Step 5: Commit**

```bash
git add sentinel-core/src/query/select.rs sentinel-core/tests/fetch_test.rs
git commit -m "feat(core): add fetch_all/fetch_one/fetch_optional to SelectQuery"
```

---

## Task 6: Add Execute Methods to InsertQuery and DeleteQuery

**Files:**
- Modify: `sentinel-core/src/query/insert.rs`
- Modify: `sentinel-core/src/query/delete.rs`
- Create: `sentinel-core/tests/execute_query_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/execute_query_test.rs`:
```rust
use sentinel_core::query::{DeleteQuery, InsertQuery};

#[test]
fn insert_build_still_works() {
    let q = InsertQuery::new("users").column("email", "alice@example.com");
    let (sql, binds) = q.build();
    assert!(sql.contains("INSERT INTO"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn delete_build_still_works() {
    let q = DeleteQuery::new("users").where_id(sentinel_core::types::Value::Int(1));
    let (sql, binds) = q.build();
    assert!(sql.contains("DELETE FROM"));
    assert_eq!(binds.len(), 1);
}

// Compile-time test: verify execute methods exist
#[allow(dead_code)]
async fn execute_api_compiles(conn: &mut sentinel_driver::Connection) {
    let q = InsertQuery::new("users").column("email", "test@test.com");
    let _rows: Vec<sentinel_driver::Row> = q.fetch_returning(conn).await.unwrap();

    let q2 = DeleteQuery::new("users").where_id(sentinel_core::types::Value::Int(1));
    let _affected: u64 = q2.execute(conn).await.unwrap();
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test execute_query_test
```

Expected: FAIL — `fetch_returning`, `execute` methods don't exist.

**Step 3: Add execute methods**

Add to the end of `impl InsertQuery` in `sentinel-core/src/query/insert.rs`:
```rust
    /// Execute this INSERT and return all rows via RETURNING clause.
    pub async fn fetch_returning(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<Vec<sentinel_driver::Row>> {
        let (sql, binds) = self.build();
        let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> =
            binds.iter().map(|v| v as &(dyn sentinel_driver::ToSql + Sync)).collect();
        Ok(conn.query(&sql, &params).await?)
    }

    /// Execute this INSERT and return the number of rows affected.
    pub async fn execute(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<u64> {
        let (sql, binds) = self.build();
        let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> =
            binds.iter().map(|v| v as &(dyn sentinel_driver::ToSql + Sync)).collect();
        Ok(conn.execute(&sql, &params).await?)
    }
```

Add to the end of `impl DeleteQuery` in `sentinel-core/src/query/delete.rs`:
```rust
    /// Execute this DELETE and return the number of rows affected.
    pub async fn execute(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<u64> {
        let (sql, binds) = self.build();
        let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> =
            binds.iter().map(|v| v as &(dyn sentinel_driver::ToSql + Sync)).collect();
        Ok(conn.execute(&sql, &params).await?)
    }
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test execute_query_test
```

Expected: 2 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/src/query/insert.rs sentinel-core/src/query/delete.rs sentinel-core/tests/execute_query_test.rs
git commit -m "feat(core): add execute/fetch_returning to InsertQuery and DeleteQuery"
```

---

## Task 7: Generate FromRow in derive(Model)

**Files:**
- Modify: `sentinel-macros/src/model/codegen.rs`
- Modify: `sentinel-macros/src/model/mod.rs`

**Step 1: Add generate_from_row to codegen.rs**

Add to `sentinel-macros/src/model/codegen.rs`:
```rust
/// Generate `from_row()` method that decodes a driver Row into the model struct.
pub fn generate_from_row(ir: &ModelIR) -> TokenStream {
    let name = &ir.struct_name;

    let field_extractions: Vec<TokenStream> = ir
        .fields
        .iter()
        .map(|f| {
            let field_name = &f.field_name;
            if f.skip {
                quote! { #field_name: std::default::Default::default() }
            } else {
                let col_name = &f.column_name;
                quote! { #field_name: row.try_get_by_name(#col_name)? }
            }
        })
        .collect();

    quote! {
        #[automatically_derived]
        impl #name {
            /// Decode a [`sentinel_driver::Row`] into this model.
            pub fn from_row(row: &sentinel_driver::Row) -> sentinel_driver::Result<Self> {
                Ok(Self {
                    #(#field_extractions,)*
                })
            }
        }
    }
}
```

**Step 2: Update model/mod.rs to call generate_from_row**

In `sentinel-macros/src/model/mod.rs`, add after `let create_method = ...`:
```rust
    let from_row = codegen::generate_from_row(&ir);
```

And add `#from_row` to the `quote!` output block.

**Step 3: Verify workspace compiles**

```bash
cargo check --workspace
```

Expected: Compiles with zero errors.

**Step 4: Commit**

```bash
git add sentinel-macros/src/model/
git commit -m "feat(macros): generate from_row() in derive(Model) for Row deserialization"
```

---

## Task 8: Generate Async Execution Methods in derive(Model)

**Files:**
- Modify: `sentinel-macros/src/model/codegen.rs`
- Modify: `sentinel-macros/src/model/mod.rs`
- Create: `sentinel-core/tests/derive_exec_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/derive_exec_test.rs`:
```rust
use sentinel_core::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,

    #[sentinel(unique)]
    pub email: String,

    pub name: Option<String>,

    #[sentinel(default = "now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// Compile-time tests: verify async methods have correct signatures
#[allow(dead_code, unused_variables)]
async fn find_all_compiles(conn: &mut sentinel_driver::Connection) {
    let users: Vec<User> = User::find_all(conn).await.unwrap();
}

#[allow(dead_code, unused_variables)]
async fn find_one_compiles(conn: &mut sentinel_driver::Connection) {
    let user: User = User::find_one(conn, &uuid::Uuid::nil()).await.unwrap();
}

#[allow(dead_code, unused_variables)]
async fn find_optional_compiles(conn: &mut sentinel_driver::Connection) {
    let user: Option<User> = User::find_optional(conn, &uuid::Uuid::nil()).await.unwrap();
}

#[allow(dead_code, unused_variables)]
async fn create_exec_compiles(conn: &mut sentinel_driver::Connection) {
    let new = NewUser {
        email: "test@test.com".into(),
        name: None,
    };
    let user: User = User::create_exec(conn, new).await.unwrap();
}

#[allow(dead_code, unused_variables)]
async fn delete_by_id_compiles(conn: &mut sentinel_driver::Connection) {
    let affected: u64 = User::delete_by_id(conn, &uuid::Uuid::nil()).await.unwrap();
}

// Actual unit test (no connection needed)
#[test]
fn derive_model_generates_execution_methods() {
    // If this test file compiles, the methods exist with correct signatures.
    assert!(true);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test derive_exec_test
```

Expected: FAIL — `find_all`, `find_one`, `find_optional`, `create_exec`, `delete_by_id` don't exist.

**Step 3: Add generate_execution_methods to codegen.rs**

Add to `sentinel-macros/src/model/codegen.rs`:
```rust
/// Generate async execution methods: find_all, find_one, find_optional, create_exec, delete_by_id.
pub fn generate_execution_methods(ir: &ModelIR) -> TokenStream {
    let name = &ir.struct_name;
    let table = &ir.table_name;
    let pk_field = &ir.fields[ir.primary_key_index];
    let pk_name = &pk_field.column_name;
    let new_name = syn::Ident::new(&format!("New{}", ir.struct_name), ir.struct_name.span());

    let insert_column_calls: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.skip && !f.has_default)
        .map(|f| {
            let col_name = &f.column_name;
            let field_name = &f.field_name;
            quote! { .column(#col_name, new.#field_name) }
        })
        .collect();

    let select_sql = format!("SELECT \"{table}\".* FROM \"{table}\"");
    let select_by_id_sql = format!("SELECT \"{table}\".* FROM \"{table}\" WHERE \"{pk_name}\" = $1");
    let delete_by_id_sql = format!("DELETE FROM \"{table}\" WHERE \"{pk_name}\" = $1");

    quote! {
        #[automatically_derived]
        impl #name {
            /// Fetch all rows from this model's table.
            pub async fn find_all(
                conn: &mut sentinel_driver::Connection,
            ) -> sentinel_core::error::Result<Vec<Self>> {
                let rows = conn.query(#select_sql, &[]).await?;
                rows.into_iter()
                    .map(|r| Self::from_row(&r).map_err(sentinel_core::error::Error::from))
                    .collect()
            }

            /// Fetch one row by primary key. Returns error if not found.
            pub async fn find_one(
                conn: &mut sentinel_driver::Connection,
                id: &(dyn sentinel_driver::ToSql + Sync),
            ) -> sentinel_core::error::Result<Self> {
                let row = conn.query_one(#select_by_id_sql, &[id]).await?;
                Self::from_row(&row).map_err(sentinel_core::error::Error::from)
            }

            /// Fetch one row by primary key. Returns None if not found.
            pub async fn find_optional(
                conn: &mut sentinel_driver::Connection,
                id: &(dyn sentinel_driver::ToSql + Sync),
            ) -> sentinel_core::error::Result<Option<Self>> {
                match conn.query_opt(#select_by_id_sql, &[id]).await? {
                    Some(row) => Ok(Some(
                        Self::from_row(&row).map_err(sentinel_core::error::Error::from)?
                    )),
                    None => Ok(None),
                }
            }

            /// Insert a new row and return the created model (via RETURNING *).
            pub async fn create_exec(
                conn: &mut sentinel_driver::Connection,
                new: #new_name,
            ) -> sentinel_core::error::Result<Self> {
                let q = sentinel_core::query::InsertQuery::new(#table)
                    #(#insert_column_calls)*;
                let rows = q.fetch_returning(conn).await?;
                let row = rows.into_iter()
                    .next()
                    .ok_or(sentinel_core::error::Error::NotFound)?;
                Self::from_row(&row).map_err(sentinel_core::error::Error::from)
            }

            /// Delete a row by primary key. Returns the number of rows deleted.
            pub async fn delete_by_id(
                conn: &mut sentinel_driver::Connection,
                id: &(dyn sentinel_driver::ToSql + Sync),
            ) -> sentinel_core::error::Result<u64> {
                Ok(conn.execute(#delete_by_id_sql, &[id]).await?)
            }
        }
    }
}
```

**Step 4: Update model/mod.rs to call generate_execution_methods**

In `sentinel-macros/src/model/mod.rs`, add after `let from_row = ...`:
```rust
    let execution_methods = codegen::generate_execution_methods(&ir);
```

And add `#execution_methods` to the `quote!` output block.

**Step 5: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test derive_exec_test
```

Expected: 1 test PASS (compile-time verification).

**Step 6: Commit**

```bash
git add sentinel-macros/src/model/ sentinel-core/tests/derive_exec_test.rs
git commit -m "feat(macros): generate async execution methods (find_all, find_one, create_exec, delete_by_id)"
```

---

## Task 9: Full Suite Verification + Clippy + Fmt

**Files:**
- Possibly fix any clippy/fmt issues across the workspace

**Step 1: Run full test suite**

```bash
cargo test --workspace
```

Expected: All tests PASS (Phase 1 + Phase 2 + Phase 3 tests).

**Step 2: Run clippy**

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Fix any warnings.

**Step 3: Check formatting**

```bash
cargo fmt --all -- --check
```

Fix any issues with `cargo fmt --all`.

**Step 4: Commit if any fixes needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Add sentinel-driver dep + re-exports | cargo check |
| 2 | ToSql for Value | 8 tests |
| 3 | #[must_use] on query builders | existing tests pass |
| 4 | Error bridge (Driver to Sentinel) | 2 tests |
| 5 | SelectQuery fetch methods | 1 test + compile check |
| 6 | InsertQuery/DeleteQuery execute methods | 2 tests + compile check |
| 7 | derive(Model) generates from_row() | cargo check |
| 8 | derive(Model) generates async execution methods | 1 test + compile check |
| 9 | Full suite + clippy + fmt | all tests + lint |

**Total: 9 tasks, ~14 new tests, 9 commits**

After Phase 3, developers can:
```rust
#[derive(Model)]
#[sentinel(table = "users")]
pub struct User { ... }

// Execute real queries:
let users = User::find_all(&mut conn).await?;
let user = User::find_one(&mut conn, &id).await?;
let user = User::create_exec(&mut conn, NewUser { email, name }).await?;
User::delete_by_id(&mut conn, &id).await?;

// Builder API with execution:
let active_users = User::find()
    .where_(User::ACTIVE.eq(true))
    .order_by(User::CREATED_AT.desc())
    .limit(10)
    .fetch_all(&mut conn).await?;
```
