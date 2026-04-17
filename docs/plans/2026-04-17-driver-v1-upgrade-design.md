# sentinel-driver v1.0.0 Upgrade — Design Document

> **GitHub Issue:** #10
> **Scope:** Full — version bump, new types, GenericClient, COPY, Portal, LISTEN/NOTIFY, query_typed()

## Goal

Upgrade sentinel-driver from v0.1.1 to v1.0.0 and expose all production-ready features at the ORM level.

## Architecture

### 1. Version Bump + Re-exports

Update `Cargo.toml` to `sentinel-driver = "1.0.0"`.

New re-exports in `sntl/src/core/mod.rs` and prelude:

- **Core:** `GenericClient`, `FromSql`
- **Config:** `LoadBalanceHosts`, `TargetSessionAttrs`, `ChannelBinding`
- **Protocol:** `Portal`, `Notification`, `SimpleQueryRow`, `SimpleQueryMessage`
- **Types:** `Json<T>` (feature-gated `with-serde-json`)
- **Locks:** `PgAdvisoryLock`, `PgAdvisoryLockGuard`
- **COPY:** `BinaryCopyEncoder`, `TextCopyEncoder`
- **Observability:** `QueryMetrics`, `ObservabilityConfig`, `PoolMetrics`

### 2. GenericClient Integration

Change all query execution methods from `&mut driver::Connection` to `&mut (impl driver::GenericClient + Send)`:

- `sntl/src/core/query/exec.rs` — SelectQuery, InsertQuery, UpdateQuery, DeleteQuery
- `sntl/src/core/query/pascal.rs` — ModelQuery FetchAll/FetchOne/FetchOptional/FetchStream
- `sntl/src/core/query/include.rs` — IncludeQuery FetchOne/FetchAll
- `sntl/src/core/transaction.rs` — Transaction wrapper
- `sntl-macros/src/model/codegen.rs` — generated find_all, find_one, find_optional, create_exec, delete_by_id

Result: queries work with both `Connection` and `PooledConnection` without `.deref_mut()`.

### 3. Value Enum — New Types

Add to `sntl/src/core/types/value.rs`:

| Variant | PG Type | Driver Type |
|---------|---------|-------------|
| `Multirange(...)` | INT4MULTIRANGE, etc. | `PgMultirange<Value>` |
| `TimeTz(...)` | TIMETZ | `PgTimeTz` |
| `MacAddr8(...)` | MACADDR8 | `PgMacAddr8` |
| `LTree(...)` | LTREE | `PgLTree` |
| `LQuery(...)` | LQUERY | `PgLQuery` |
| `Cube(...)` | CUBE | `PgCube` |

Plus:
- `From<Json<T>>` conversion for typed JSON wrapper
- Missing array OID match arms (JSON[], JSONB[], TIMESTAMP[], DATE[], TIME[], geometric[])
- Optional `with-time` feature forwarding to driver

### 4. ORM-level Wrappers

#### 4a. COPY Protocol

Macro-generated `copy_in()` on Model:

```rust
User::copy_in(&mut conn, users).await?
```

Generates COPY SQL from Model trait metadata (TABLE, columns).

#### 4b. Portal/Cursor

PascalCase cursor wrapper:

```rust
let mut portal = User::Find().Cursor(&mut conn).await?;
let batch = portal.Fetch(100, &mut conn).await?;
portal.Close(&mut conn).await?;
```

#### 4c. LISTEN/NOTIFY

Re-export only — driver API is ergonomic as-is.

#### 4d. query_typed()

`.Typed()` method on ModelQuery that uses column OIDs to skip prepare:

```rust
User::Find().Where(...).Typed().FetchAll(&mut conn).await?
```

#### 4e. Advisory Locks

Re-export only — driver API is ergonomic as-is.

### 5. Testing Strategy

| Layer | Tests |
|-------|-------|
| Unit | Value new variants: From, ToSql, PartialEq, Display |
| Unit | New re-exports accessible from prelude |
| Compile | GenericClient — FetchAll with PooledConnection compiles |
| Integration | COPY bulk insert + verify count |
| Integration | Portal/Cursor batch fetch |
| Integration | query_typed() same results as normal |
| Integration | LISTEN/NOTIFY round-trip |
| Integration | New types roundtrip: multirange, TIMETZ, MACADDR8, LTree, Cube |
| Integration | GenericClient — same query via Connection vs PooledConnection |

Coverage ignore: existing `query/exec.rs`, `query/pascal.rs`, `query/include.rs` + new async COPY/Portal wrappers.

### 6. Estimated Size

~1500-2000 lines — single PR.
