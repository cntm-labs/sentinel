# sentinel-driver v1.0.0 Upgrade — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Upgrade sentinel-driver from v0.1.1 to v1.0.0 and expose all production-ready features (GenericClient, new types, COPY, Portal, query_typed, LISTEN/NOTIFY) at the ORM level.

**Architecture:** Bump dependency, switch all `&mut Connection` params to `&mut (impl GenericClient + Send)`, add new Value variants for v1.0.0 types, add ORM-level wrappers for COPY/Portal/query_typed, re-export new driver types. Single PR closing issue #10.

**Tech Stack:** Rust, sentinel-driver v1.0.0, proc macros (syn/quote)

---

### Task 1: Version bump + compilation check

**Files:**
- Modify: `Cargo.toml` (workspace root, line 35)
- Modify: `sntl/Cargo.toml` (line 14)

**Step 1: Update workspace dependency**

In `Cargo.toml` (workspace root), change line 35:
```toml
sentinel-driver = "1.0.0"
```

In `sntl/Cargo.toml`, change line 14:
```toml
driver = { package = "sentinel-driver", version = "1.0.0", features = ["with-rust-decimal"] }
```

**Step 2: Check compilation**

Run: `cargo check --workspace`
Expected: PASS (v1.0.0 should be backward compatible with v0.1.1 API)
If FAIL: Fix any breaking changes before proceeding.

**Step 3: Run existing tests**

Run: `cargo test --workspace`
Expected: All existing tests still pass.

**Step 4: Commit**

```bash
git add Cargo.toml sntl/Cargo.toml Cargo.lock
git commit -m "chore: bump sentinel-driver to v1.0.0"
```

---

### Task 2: Re-export new driver types

**Files:**
- Modify: `sntl/src/core/mod.rs`
- Modify: `sntl/src/core/prelude.rs`
- Modify: `sntl/src/lib.rs`
- Test: `sntl/tests/prelude_test.rs`

**Step 1: Write failing test**

Add to `sntl/tests/prelude_test.rs`:

```rust
#[test]
fn v1_types_accessible() {
    // GenericClient trait is accessible
    fn _assert_generic_client<T: sntl::driver::GenericClient>() {}

    // Config enums
    let _ = sntl::core::LoadBalanceHosts::Disable;
    let _ = sntl::core::TargetSessionAttrs::Any;
    let _ = sntl::core::ChannelBinding::Prefer;

    // Protocol types
    let _ = std::mem::size_of::<sntl::core::Portal>();
    let _ = std::mem::size_of::<sntl::core::Notification>();
    let _ = std::mem::size_of::<sntl::core::SimpleQueryRow>();
    let _ = std::mem::size_of::<sntl::core::SimpleQueryMessage>();

    // Observability
    let _ = std::mem::size_of::<sntl::core::PoolMetrics>();

    // Advisory locks
    let _ = std::mem::size_of::<sntl::core::PgAdvisoryLock>();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package sntl --test prelude_test`
Expected: FAIL — types not found in `sntl::core`

**Step 3: Add re-exports to `sntl/src/core/mod.rs`**

Add after existing driver re-exports:

```rust
// Re-export new v1.0.0 driver types
pub use driver::{GenericClient, LoadBalanceHosts, TargetSessionAttrs, ChannelBinding};
pub use driver::{Portal, Notification, SimpleQueryRow, SimpleQueryMessage};
pub use driver::{PgAdvisoryLock, PgAdvisoryLockGuard};
pub use driver::{BinaryCopyEncoder, TextCopyEncoder};
pub use driver::{PoolMetrics};
pub use driver::FromSql;
```

**Step 4: Add to prelude**

In `sntl/src/core/prelude.rs`, add to the driver re-exports section:

```rust
pub use driver::{GenericClient, LoadBalanceHosts, TargetSessionAttrs, ChannelBinding};
pub use driver::{Portal, Notification, SimpleQueryRow, SimpleQueryMessage};
pub use driver::{PgAdvisoryLock, PgAdvisoryLockGuard};
pub use driver::PoolMetrics;
```

**Step 5: Run tests**

Run: `cargo test --package sntl --test prelude_test`
Expected: PASS

**Step 6: Commit**

```bash
git add sntl/src/core/mod.rs sntl/src/core/prelude.rs sntl/tests/prelude_test.rs
git commit -m "feat: re-export sentinel-driver v1.0.0 types"
```

---

### Task 3: GenericClient in query execution

**Files:**
- Modify: `sntl/src/core/query/exec.rs`
- Modify: `sntl/src/core/query/pascal.rs`
- Modify: `sntl/src/core/query/include.rs`
- Modify: `sntl-macros/src/model/codegen.rs`
- Test: existing tests must still pass

**Step 1: Run existing tests to confirm baseline**

Run: `cargo test --workspace`
Expected: PASS

**Step 2: Update `exec.rs`**

Replace all `conn: &mut driver::Connection` with `conn: &mut (impl driver::GenericClient + Send)` in:
- `SelectQuery::fetch_all`
- `SelectQuery::fetch_one`
- `SelectQuery::fetch_optional`
- `SelectQuery::fetch_stream`
- `InsertQuery::fetch_returning`
- `InsertQuery::execute`
- `UpdateQuery::fetch_returning`
- `UpdateQuery::execute`
- `DeleteQuery::execute`

Example change:
```rust
pub async fn fetch_all(
    self,
    conn: &mut (impl driver::GenericClient + Send),
) -> crate::core::error::Result<Vec<driver::Row>> {
    let (sql, binds) = self.build();
    Ok(conn.query(&sql, &to_params(&binds)).await?)
}
```

**NOTE for `fetch_stream`:** `GenericClient` may not have `query_stream()`. Check if it does. If not, keep `fetch_stream` on `&mut driver::Connection` only and add a separate impl block.

**Step 3: Update `pascal.rs`**

Same change for `ModelQuery<M>`:
- `FetchAll`, `FetchOne`, `FetchOptional`, `FetchStream`

```rust
pub async fn FetchAll(
    self,
    conn: &mut (impl driver::GenericClient + Send),
) -> crate::core::error::Result<Vec<driver::Row>> {
    self.inner.fetch_all(conn).await
}
```

**Step 4: Update `include.rs`**

Both `FetchOne` and `FetchAll` on `IncludeQuery<M, S>`:

```rust
pub async fn FetchOne(
    self,
    conn: &mut (impl driver::GenericClient + Send),
) -> crate::core::error::Result<WithRelations<M, S>> {
```

Also update internal `conn.query(...)` calls — these should already work since `GenericClient` has `query()`.

**Step 5: Update macro codegen**

In `sntl-macros/src/model/codegen.rs`, change `generate_execution_methods()` to emit:

```rust
conn: &mut (impl sntl::driver::GenericClient + Send)
```

instead of:

```rust
conn: &mut sntl::core::Connection
```

For all generated methods: `find_all`, `find_one`, `find_optional`, `create_exec`, `delete_by_id`.

**Step 6: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass. Existing tests use `&mut Connection` which implements `GenericClient`.

**Step 7: Commit**

```bash
git add sntl/src/core/query/exec.rs sntl/src/core/query/pascal.rs sntl/src/core/query/include.rs sntl-macros/src/model/codegen.rs
git commit -m "feat: use GenericClient trait for all query execution methods"
```

---

### Task 4: New Value variants

**Files:**
- Modify: `sntl/src/core/types/value.rs`
- Test: `sntl/tests/value_test.rs`

**Step 1: Write failing tests**

Add to `sntl/tests/value_test.rs`:

```rust
#[test]
fn value_from_macaddr8() {
    let v = Value::MacAddr8([1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(matches!(v, Value::MacAddr8(_)));
}

#[test]
fn value_timetz() {
    let v = Value::TimeTz(sntl::driver::types::timetz::PgTimeTz {
        time: chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        offset: 0,
    });
    assert!(matches!(v, Value::TimeTz(_)));
}

#[test]
fn value_ltree() {
    let v = Value::LTree(sntl::driver::types::ltree::PgLTree("top.science".into()));
    assert!(matches!(v, Value::LTree(_)));
}

#[test]
fn value_lquery() {
    let v = Value::LQuery(sntl::driver::types::ltree::PgLQuery("*.science.*".into()));
    assert!(matches!(v, Value::LQuery(_)));
}

#[test]
fn value_cube() {
    let v = Value::Cube(sntl::driver::types::cube::PgCube {
        coordinates: vec![1.0, 2.0, 3.0],
        is_point: true,
    });
    assert!(matches!(v, Value::Cube(_)));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test value_test`
Expected: FAIL — variants don't exist

**Step 3: Add new variants to Value enum**

Add after the existing `MacAddr([u8; 6])` variant:

```rust
    MacAddr8([u8; 8]),
```

Add after existing `Time(NaiveTime)`:

```rust
    TimeTz(driver::types::timetz::PgTimeTz),
```

Add new section after Geometric:

```rust
    // === Extension types ===
    LTree(driver::types::ltree::PgLTree),
    LQuery(driver::types::ltree::PgLQuery),
    Cube(driver::types::cube::PgCube),
```

Add new section after existing ranges:

```rust
    // === Multiranges (PG 14+) ===
    Int4Multirange(driver::types::multirange::PgMultirange<i32>),
    Int8Multirange(driver::types::multirange::PgMultirange<i64>),
    NumMultirange(driver::types::multirange::PgMultirange<rust_decimal::Decimal>),
    TsMultirange(driver::types::multirange::PgMultirange<NaiveDateTime>),
    TsTzMultirange(driver::types::multirange::PgMultirange<DateTime<Utc>>),
    DateMultirange(driver::types::multirange::PgMultirange<NaiveDate>),
```

**Step 4: Add From impls, PartialEq, Display, Debug, ToSql for new variants**

Follow the existing patterns exactly. Add `oid()` match arms, `to_sql()` delegation, `Display` formatting, `PartialEq` arms, `is_*` / `as_*` methods.

**Step 5: Add missing array OID match arms**

In the `oid()` method's Array match, add:

```rust
Value::Json(_) => driver::Oid::JSONB_ARRAY,
Value::TimestampNaive(_) => driver::Oid::TIMESTAMP_ARRAY,
Value::Date(_) => driver::Oid::DATE_ARRAY,
Value::Time(_) => driver::Oid::TIME_ARRAY,
Value::Point(_) => driver::Oid::POINT_ARRAY,
Value::Circle(_) => driver::Oid::CIRCLE_ARRAY,
```

(Verify OID constants exist in v1.0.0 driver before adding.)

**Step 6: Run tests**

Run: `cargo test --package sntl --test value_test`
Expected: PASS

Run: `cargo test --workspace`
Expected: All PASS

**Step 7: Commit**

```bash
git add sntl/src/core/types/value.rs sntl/tests/value_test.rs
git commit -m "feat: add Value variants for v1.0.0 types (multirange, TIMETZ, MACADDR8, LTree, Cube)"
```

---

### Task 5: Transaction GenericClient update

**Files:**
- Modify: `sntl/src/core/transaction.rs`
- Test: existing transaction tests must pass

**Step 1: Update Transaction to use GenericClient**

The `Transaction` struct currently holds `&'c mut Connection`. Since `Connection` implements `GenericClient`, and `Transaction` wraps a connection, update the `query()` and `execute()` methods to delegate through the connection's `GenericClient` impl.

No structural change needed if `Connection` already implements `GenericClient` — the wrapper methods just call through. The `begin()`/`commit()`/`rollback()` methods are connection-specific and stay as `Connection`.

Keep `Transaction::begin()` taking `&mut Connection` (transactions require a real connection, not a trait). But `Transaction::query()` and `Transaction::execute()` already delegate to `self.conn.query()` which works through `GenericClient`.

**Step 2: Run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 3: Commit (if changes needed)**

```bash
git add sntl/src/core/transaction.rs
git commit -m "refactor: align Transaction with GenericClient"
```

---

### Task 6: ORM COPY wrapper

**Files:**
- Create: `sntl/src/core/copy.rs`
- Modify: `sntl/src/core/mod.rs`
- Modify: `sntl-macros/src/model/codegen.rs`
- Test: `sntl/tests/copy_test.rs`
- Integration: `sntl/tests/pg_copy_test.rs`

**Step 1: Write failing test**

Create `sntl/tests/copy_test.rs`:

```rust
use sntl::core::Model;

// Verify copy_columns() is generated
#[test]
fn model_has_copy_columns() {
    // Model trait should provide columns metadata for COPY
    // This test validates the macro generates the right SQL
}
```

**Step 2: Create `sntl/src/core/copy.rs`**

```rust
//! COPY protocol support for bulk operations.

use crate::core::model::Model;
use crate::core::types::Value;

/// Build a COPY IN SQL statement from Model metadata.
///
/// Returns: `COPY "table" ("col1", "col2", ...) FROM STDIN BINARY`
pub fn copy_in_sql<M: Model>() -> String {
    let cols: Vec<&str> = M::columns().iter().map(|c| c.name).collect();
    let col_list = cols
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(", ");
    format!("COPY \"{}\" ({}) FROM STDIN BINARY", M::TABLE, col_list)
}
```

Register in `sntl/src/core/mod.rs`:
```rust
pub mod copy;
```

**Step 3: Write integration test**

Create `sntl/tests/pg_copy_test.rs`:
```rust
#[macro_use]
mod pg_helpers;

use sntl::prelude::*;
use sntl::{Model, sentinel};

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[tokio::test]
async fn copy_in_sql_generates_correct_statement() {
    let sql = sntl::core::copy::copy_in_sql::<User>();
    assert!(sql.contains("COPY"));
    assert!(sql.contains("users"));
    assert!(sql.contains("FROM STDIN BINARY"));
}
```

**Step 4: Run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 5: Commit**

```bash
git add sntl/src/core/copy.rs sntl/src/core/mod.rs sntl/tests/copy_test.rs sntl/tests/pg_copy_test.rs
git commit -m "feat: add COPY protocol support with copy_in_sql helper"
```

---

### Task 7: Portal/Cursor PascalCase wrapper

**Files:**
- Create: `sntl/src/core/query/cursor.rs`
- Modify: `sntl/src/core/query/mod.rs`
- Modify: `sntl/src/core/query/pascal.rs`
- Test: `sntl/tests/cursor_test.rs`

**Step 1: Write failing test**

Create `sntl/tests/cursor_test.rs`:

```rust
use sntl::core::query::CursorQuery;

#[test]
fn cursor_query_builds_sql() {
    let q: CursorQuery = CursorQuery::from_table("users");
    let (sql, _) = q.Build();
    assert!(sql.contains("users"));
}
```

**Step 2: Create `sntl/src/core/query/cursor.rs`**

```rust
use crate::core::query::SelectQuery;
use crate::core::types::Value;
use crate::core::expr::{Expr, OrderExpr};

/// Cursor-based query builder for incremental fetching via Portal.
///
/// Usage:
/// ```ignore
/// let mut portal = User::Find().Cursor(&mut conn).await?;
/// let batch = portal.Fetch(100, &mut conn).await?;
/// portal.Close(&mut conn).await?;
/// ```
#[must_use = "cursor does nothing until .Bind() is called"]
pub struct CursorQuery {
    inner: SelectQuery,
}

impl CursorQuery {
    pub fn from_table(table: &str) -> Self {
        Self {
            inner: SelectQuery::new(table),
        }
    }

    pub fn from_select(select: SelectQuery) -> Self {
        Self { inner: select }
    }

    #[allow(non_snake_case)]
    pub fn Where(mut self, expr: Expr) -> Self {
        self.inner = self.inner.where_(expr);
        self
    }

    #[allow(non_snake_case)]
    pub fn OrderBy(mut self, order: OrderExpr) -> Self {
        self.inner = self.inner.order_by(order);
        self
    }

    #[allow(non_snake_case)]
    pub fn Build(&self) -> (String, Vec<Value>) {
        self.inner.build()
    }

    /// Bind a server-side portal for incremental fetching.
    #[allow(non_snake_case)]
    pub async fn Bind(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<driver::Portal> {
        let (sql, binds) = self.inner.build();
        let params: Vec<&(dyn driver::ToSql + Sync)> =
            binds.iter().map(|v| v as &(dyn driver::ToSql + Sync)).collect();
        Ok(conn.bind_portal(&sql, &params).await?)
    }
}
```

Register in `query/mod.rs`:
```rust
pub mod cursor;
pub use cursor::CursorQuery;
```

Add `.Cursor()` on `ModelQuery<M>` in `pascal.rs`:
```rust
#[allow(non_snake_case)]
pub fn Cursor(self) -> CursorQuery {
    CursorQuery::from_select(self.inner)
}
```

**Step 3: Run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 4: Commit**

```bash
git add sntl/src/core/query/cursor.rs sntl/src/core/query/mod.rs sntl/src/core/query/pascal.rs sntl/tests/cursor_test.rs
git commit -m "feat: add CursorQuery for Portal-based incremental fetching"
```

---

### Task 8: query_typed() — `.Typed()` on ModelQuery

**Files:**
- Modify: `sntl/src/core/query/pascal.rs`
- Modify: `sntl/src/core/model.rs` (add `column_oids()` to Model trait)
- Modify: `sntl-macros/src/model/codegen.rs` (generate `column_oids()`)
- Test: `sntl/tests/typed_query_test.rs`

**Step 1: Write failing test**

Create `sntl/tests/typed_query_test.rs`:

```rust
use sntl::core::query::ModelQuery;
use sntl::core::Column;

#[test]
fn typed_query_builds_same_sql() {
    let q: ModelQuery = ModelQuery::from_table("users");
    let typed_q = q.Where(Column::new("users", "id").eq(1)).Typed();
    let (sql, _) = typed_q.Build();
    assert!(sql.contains("users"));
    assert!(sql.contains("WHERE"));
}
```

**Step 2: Add `Typed()` method**

Add `TypedQuery` struct wrapping `SelectQuery` + collected OIDs, with the same `Build()` method plus a `FetchAll` that calls `conn.query_typed()`.

**Step 3: Run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 4: Commit**

```bash
git add sntl/src/core/query/pascal.rs sntl/src/core/model.rs sntl-macros/src/model/codegen.rs sntl/tests/typed_query_test.rs
git commit -m "feat: add .Typed() for query_typed() skip-prepare optimization"
```

---

### Task 9: Integration tests for new types + features

**Files:**
- Modify: `tests/integration/setup.sql` (add new columns)
- Modify: `sntl/tests/pg_value_roundtrip_test.rs` (new type roundtrips)
- Create: `sntl/tests/pg_generic_client_test.rs`

**Step 1: Update setup.sql**

Add columns to `type_roundtrip` table:

```sql
    macaddr8_col    MACADDR8,
    timetz_col      TIMETZ,
```

Note: LTREE, CUBE require PG extensions — add with `CREATE EXTENSION IF NOT EXISTS` or skip for now and test via Custom value.

**Step 2: Write roundtrip tests**

Add to `sntl/tests/pg_value_roundtrip_test.rs`:

```rust
#[tokio::test]
async fn roundtrip_macaddr8() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    let row = roundtrip_one(&mut conn, "macaddr8_col", Value::MacAddr8([1,2,3,4,5,6,7,8])).await;
    let val: [u8; 8] = row.try_get_by_name("macaddr8_col").unwrap();
    assert_eq!(val, [1,2,3,4,5,6,7,8]);
}

#[tokio::test]
async fn roundtrip_timetz() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    let timetz = sntl::driver::types::timetz::PgTimeTz {
        time: chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
        offset: 0,
    };
    let row = roundtrip_one(&mut conn, "timetz_col", Value::TimeTz(timetz)).await;
    // Verify it roundtrips
}
```

**Step 3: Write GenericClient integration test**

Create `sntl/tests/pg_generic_client_test.rs`:

```rust
#[macro_use]
mod pg_helpers;

use sntl::prelude::*;

#[tokio::test]
async fn query_via_pooled_connection() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let pool = sntl::driver::Pool::new(config, Default::default());
    let mut conn = pool.acquire().await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // PooledConnection should work with all query builders
    InsertQuery::new("users")
        .column("name", "Pool User")
        .column("email", "pool@test.com")
        .no_returning()
        .execute(&mut conn)
        .await
        .unwrap();

    let rows = SelectQuery::new("users").fetch_all(&mut conn).await.unwrap();
    assert_eq!(rows.len(), 1);
}
```

**Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: PASS (integration tests skip without DATABASE_URL)

**Step 5: Commit**

```bash
git add tests/integration/setup.sql sntl/tests/pg_value_roundtrip_test.rs sntl/tests/pg_generic_client_test.rs
git commit -m "test: integration tests for v1.0.0 types and GenericClient"
```

---

### Task 10: Update coverage config + Clippy + format + final PR

**Files:**
- Modify: `.github/workflows/codecov.yml` (add new async files to ignore)
- All modified files

**Step 1: Update coverage ignore**

Add `copy\.rs|cursor\.rs` to the ignore regex if they contain async execution code.

**Step 2: Format**

Run: `cargo fmt --all`

**Step 3: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Fix any warnings.

**Step 4: Full test suite**

Run: `cargo test --workspace`
Expected: All PASS

**Step 5: Create PR**

```bash
gh pr create --title "feat: upgrade sentinel-driver to v1.0.0 — GenericClient, new types, COPY, Portal" \
  --body "Closes #10" --assignee MrBT-nano
```
