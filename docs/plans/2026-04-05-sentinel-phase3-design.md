# Sentinel ORM — Phase 3: Connection + Driver Integration Design

**Goal:** Integrate sentinel-core with sentinel-driver so that derive(Model) generates executable async methods that query a real PostgreSQL database.

**Approach:** Direct dependency — sentinel-core depends on sentinel-driver directly. No abstraction trait. PG-only optimization.

**Driver version:** sentinel-driver v0.1.0 (crates workspace, all 4 issues resolved: array types, cancel query, per-query timeout, realtime health check)

---

## Architecture

```
User Code
    ↓
#[derive(Model)] struct User { ... }
    ↓  (generates)
├── Model trait impl         (TABLE, PRIMARY_KEY, columns)     [Phase 2 ✓]
├── Column constants         (User::EMAIL, User::ID)           [Phase 2 ✓]
├── NewModel struct           (NewUser)                         [Phase 2 ✓]
├── FromRow impl             (Row → User deserialization)       [Phase 3]
├── Async execution methods  (find_all, find_one, create, etc) [Phase 3]
└── Query builder .fetch()   (SelectQuery → Vec<User>)         [Phase 3]
    ↓
sentinel-core query builders → build() → (SQL, Vec<Value>)
    ↓
Value → ToSql bridge                                            [Phase 3]
    ↓
sentinel-driver::Connection::query(sql, &[&dyn ToSql])
    ↓
PostgreSQL (binary wire protocol)
```

## 1. Dependency Wiring

sentinel-core adds sentinel-driver as a workspace dependency. Re-exports key driver types:

```rust
pub use sentinel_driver::{Connection, Pool, Config, SslMode};
pub use sentinel_driver::{IsolationLevel, TransactionConfig, CancelToken};
```

## 2. Value → ToSql Bridge

`impl sentinel_driver::ToSql for Value` — delegates to driver's builtin encoders per variant (Bool, Int, BigInt, Double, Text, Uuid, Timestamp, Bytes). Null handled by caller.

Query execution flow:
1. Builder `.build()` → `(String, Vec<Value>)`
2. `.fetch_all(conn)` converts `Vec<Value>` → `Vec<&dyn ToSql>`
3. Calls `conn.query(sql, &params)` → `Vec<Row>`
4. `FromRow::from_row()` → `Vec<Model>`

## 3. FromRow Generation

`derive(Model)` generates `impl sentinel_driver::FromRow for Model`:
- Uses `row.try_get_by_name::<T>(column_name)` per field
- Respects `#[sentinel(column = "...")]` rename
- Skipped fields (`#[sentinel(skip)]`) use `Default::default()`

## 4. Async Execution Methods

`derive(Model)` generates on the model struct:

| Method | Returns | SQL Pattern |
|--------|---------|-------------|
| `find_all(conn)` | `Vec<Self>` | `SELECT * FROM table` |
| `find_one(conn, id)` | `Self` | `SELECT * FROM table WHERE pk = $1` |
| `find_optional(conn, id)` | `Option<Self>` | `SELECT * FROM table WHERE pk = $1` |
| `create(conn, new)` | `Self` | `INSERT ... RETURNING *` |
| `delete_by_id(conn, id)` | `u64` | `DELETE ... WHERE pk = $1` |

All methods take `conn: &mut sentinel_driver::Connection`.

## 5. Query Builder Execution

Existing query builders gain execution methods:

| Method | Returns | Used By |
|--------|---------|---------|
| `.fetch_all(conn)` | `Vec<T: FromRow>` | SelectQuery |
| `.fetch_one(conn)` | `T: FromRow` | SelectQuery |
| `.fetch_optional(conn)` | `Option<T: FromRow>` | SelectQuery |
| `.execute(conn)` | `u64` (rows affected) | InsertQuery, UpdateQuery, DeleteQuery |

## 6. #[must_use] on All Builders

All query builder structs get `#[must_use]` to warn when a query is constructed but never executed.

## 7. Testing Strategy

- Query builder `.build()` tests: unchanged (string assertion)
- `FromRow` tests: construct mock Row from raw bytes, verify deserialization
- `ToSql for Value` tests: round-trip encode → decode
- Execution method tests: verify SQL + params correctness (no real DB)

## Not In Scope

- Pool wrapper with tracing/metrics
- `update()` execution (needs builder improvements)
- Type-state relations (Phase 4)
- Transaction/reducer (Phase 5)
- Real PG integration tests (Phase 3.5)
