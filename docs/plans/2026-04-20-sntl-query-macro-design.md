# Design: `sntl::query!()` Compile-Time SQL Validation Macro

**Date:** 2026-04-20
**Status:** Draft
**Target release:** v0.2.0
**Goals:** Performance ≥ sqlx, DX ≥ Prisma, leverage sentinel-driver advantages
**Owner:** MrBT-nano

---

## 1. Problem Statement

sqlx's killer feature is `query!` / `query_as!` — compile-time SQL validation against a live PostgreSQL server. Users commit to sqlx largely because of this moat.

Sentinel currently offers stronger type-state relations (compile-time N+1 prevention) and a custom driver with protocol-level advantages (pipelining, COPY, two-tier statement cache, correct SCRAM-SASLprep), but lacks compile-time SQL validation. Without it, Sentinel cannot displace sqlx for teams that prize that guarantee.

This design specifies a macro family that matches sqlx's surface, exceeds it in developer experience, and exploits sentinel-driver capabilities sqlx cannot match.

### North-star metrics

- **Compile time:** ≥ 33% faster than sqlx on cold build; ≥ 6× faster on incremental
- **Runtime:** identical for single queries, 3× faster for pipelined queries, 10–50× faster for bulk inserts
- **DX:** compile errors with 3 fix suggestions each; zero nullable annotations in 95% of queries
- **CI ergonomics:** `SENTINEL_OFFLINE=true` alone is sufficient (no DB needed)

---

## 2. Scope

### In scope (v0.2)

- Macros: `sntl::query!`, `sntl::query_as!`, `sntl::query_scalar!`, `sntl::query_file!`, `sntl::query_file_as!`, plus `_unchecked` variants
- Sentinel-unique macro: `sntl::query_pipeline!` (multiple queries in one round-trip)
- New derive: `#[derive(FromRow)]`
- New crate: `sntl-schema` (parser, cache, config, nullability engine)
- New crate: `sntl-cli` with `sntl prepare`, `sntl check`, `sntl doctor`
- Cache directory `.sentinel/` with `schema.toml` snapshot
- Nullability inference from schema + JOIN propagation + expression rules, with explicit override syntax

### Out of scope (deferred)

- Migrations CLI (`sntl migrate …`) → v0.3
- Schema introspection reverse-generation (`sntl introspect`, `sntl generate`, `sntl migrate diff`) → v0.4
- LSP / IDE integration (`sntl lsp`) → v0.4
- `sntl studio` GUI → stretch (v0.5+)
- MySQL / SQLite support → never (Sentinel is PG-only by design)

---

## 3. Architecture

### 3.1 Macro expansion pipeline

```
┌─────────────────────────────────────────────────────────────┐
│ Stage 1: PARSE                                              │
│   Tokens → SQL string + optional target type + overrides    │
│   Implementation: syn crate                                 │
├─────────────────────────────────────────────────────────────┤
│ Stage 2: RESOLVE                                            │
│   Normalize SQL → hash → lookup .sentinel/queries/{hash}.json│
│   Miss + online  → connect PG → PREPARE → write cache entry │
│   Miss + offline → compile error with fix hint              │
├─────────────────────────────────────────────────────────────┤
│ Stage 3: VALIDATE                                           │
│   Parse SQL with sqlparser-rs → track column origins        │
│   Cross-ref .sentinel/schema.toml → nullable inference      │
│   If target type T given → validate Model/Partial/FromRow   │
├─────────────────────────────────────────────────────────────┤
│ Stage 4: CODEGEN                                            │
│   Emit: struct literal or T::from_row call                  │
│   + bind params with correct OIDs                           │
│   + route through sentinel-driver's query_typed() path      │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Crate layout

```
sentinel/
├── sntl/                    # existing — main API
├── sntl-macros/             # extended: add query!, query_as!, FromRow derive
├── sntl-schema/             # NEW: shared library between macros + CLI
│   ├── parser.rs            # sqlparser-rs wrapper, column-origin tracker
│   ├── nullable.rs          # nullability inference engine
│   ├── cache.rs             # .sentinel/ read/write
│   └── config.rs            # sentinel.toml loader with env override
├── sntl-cli/                # extended from stub: sntl prepare/check/doctor
└── sntl-migrate/            # existing stub; populated in v0.3
```

`sntl-schema` is a regular crate (not proc-macro) so both macros and CLI can depend on it, and so it is unit-testable without macro expansion overhead.

### 3.3 Runtime integration

Validated macros (`query!`, `query_as!`, `query_scalar!`, `query_file!`, `query_file_as!`) emit calls to `GenericClient::query_typed()` with the parameter OIDs resolved at compile time from the cache entry. This eliminates the Prepare round-trip that sqlx's default path pays on first execution of each statement.

Unchecked variants (`query_unchecked!`, `query_as_unchecked!`) cannot know OIDs at compile time; they lower to the standard `GenericClient::query()` path, which still benefits from sentinel-driver's two-tier statement cache on subsequent executions.

`query_pipeline!` lowers to a single `sentinel_driver::PipelineBatch`, dispatching all member queries in one round-trip with per-query OIDs from their individual cache entries.

---

## 4. Configuration

### 4.1 `sentinel.toml` (workspace root, committed)

```toml
[database]
# Local dev. Do not embed passwords; use "${VAR}" interpolation.
url = "postgres://localhost:5432/myapp_dev"

[offline]
# Default true in CI when .sentinel/ is present.
enabled = false

[schema]
path = ".sentinel/schema.toml"
dialect = "postgres-16"       # postgres-13..17 (affects feature set)

[macros]
strict_nullable = true        # warn on unknown expressions → treat as nullable
deny_warnings = false         # promote macro warnings to errors

[cache]
dir = ".sentinel"
```

### 4.2 Precedence (highest wins)

1. CLI flag (`--database-url`, `--workspace`)
2. Env var (`SENTINEL_DATABASE_URL`, `SENTINEL_OFFLINE`, `SENTINEL_WORKSPACE`)
3. `sentinel.local.toml` (gitignored, developer overrides)
4. `sentinel.toml` (committed, team defaults)
5. Built-in defaults

### 4.3 CI flow

```yaml
# No DB spin-up needed
- run: cargo build
  env:
    SENTINEL_OFFLINE: "true"   # uses committed .sentinel/
```

---

## 5. Macro API

### 5.1 Anonymous record

```rust
let row = sntl::query!(
    "SELECT id, email FROM users WHERE id = $1",
    user_id,
)
.fetch_one(&conn)
.await?;
row.id      // Uuid        (from schema: non-null uuid)
row.email   // String      (from schema: non-null text)
```

### 5.2 Typed struct with auto-detected path

```rust
let user = sntl::query_as!(
    User,                       // Model | Partial | FromRow
    "SELECT id, email, created_at FROM users WHERE id = $1",
    user_id,
)
.fetch_one(&conn)
.await?;
```

Auto-detection rules at compile time:

- If `T: sntl::Model` → strict validation: each output column must match a field declared in `Model::COLUMNS`, with matching PG type and nullability.
- Else if `T: sntl::Partial<_>` → validate against partial column subset.
- Else if `T: sntl::FromRow` → duck-type validation: output columns must cover all struct fields; extra columns are ignored.
- Else → compile error listing the three options with `help:` lines.

### 5.3 Single column

```rust
let count = sntl::query_scalar!(
    "SELECT count(*) FROM users WHERE active = $1",
    true,
)
.fetch_one(&conn)
.await?;   // i64 (from schema: count()→bigint non-null)
```

### 5.4 From file

```rust
let stats = sntl::query_file_as!(
    UserStats,
    "queries/user_stats.sql",   // relative to CARGO_MANIFEST_DIR
    user_id,
)
.fetch_all(&conn)
.await?;
```

### 5.5 Nullability overrides

```rust
sntl::query!(
    "SELECT id, my_udf(x) AS y FROM t WHERE id = $1",
    id,
    non_null = [y],             // UDF always returns non-null
    nullable = [],              // no forced-nullable columns
)
```

Override validation:

- Unknown column name → compile error.
- Overriding a PK or schema-declared non-null column to nullable → warning.

### 5.6 Unchecked variants (escape hatches)

```rust
sntl::query_unchecked!("SELECT * FROM dynamic_view", arg);
sntl::query_as_unchecked!(User, "…", arg);
```

No cache lookup, no validation. Useful during migration into Sentinel or for genuinely dynamic SQL.

### 5.7 Sentinel-unique: pipelined queries

```rust
let (users, posts, comments) = sntl::query_pipeline!(
    &conn,
    users: "SELECT * FROM users WHERE id = $1" using User,
    posts: "SELECT * FROM posts WHERE user_id = $1" using Post,
    comments: "SELECT * FROM comments WHERE user_id = $1" using Comment,
    user_id,
)
.await?;
```

Expands to a single `PipelineBatch` round-trip through sentinel-driver. Each inner query is individually validated at compile time with the same rules as `query_as!`.

### 5.8 Fetch methods

Identical to sqlx:

- `.fetch_one(&conn) -> Result<T>`
- `.fetch_optional(&conn) -> Result<Option<T>>`
- `.fetch_all(&conn) -> Result<Vec<T>>`
- `.fetch_stream(&conn) -> impl Stream<Item = Result<T>>`
- `.execute(&conn) -> Result<u64>` (rows affected, for DML without RETURNING)

---

## 6. Cache Format

### 6.1 Directory layout

```
.sentinel/
├── .version                      # "1" — cache format version
├── schema.toml                   # DB schema snapshot (shared)
├── queries/
│   ├── a3f7c2e9b1d4.json         # hash(normalized_sql) per file
│   └── …
└── .gitattributes                # mark json files as linguist-generated
```

Commit the entire directory so CI can build with `SENTINEL_OFFLINE=true`.

### 6.2 `schema.toml` (partial example)

```toml
version = 1
postgres_version = "16.2"
generated_at = "2026-04-20T10:30:00Z"
source = "postgres://localhost:5432/myapp_dev"    # password stripped

[[tables]]
name = "users"
schema = "public"

  [[tables.columns]]
  name = "id"
  pg_type = "uuid"
  oid = 2950
  nullable = false
  default = "gen_random_uuid()"
  primary_key = true

  [[tables.columns]]
  name = "email"
  pg_type = "text"
  oid = 25
  nullable = false
  unique = true

  [[tables.columns]]
  name = "deleted_at"
  pg_type = "timestamptz"
  oid = 1184
  nullable = true

  [[tables.foreign_keys]]
  columns = ["team_id"]
  references = { table = "teams", columns = ["id"] }
  on_delete = "cascade"

[[enums]]
name = "user_status"
values = ["active", "suspended", "deleted"]
oid = 16841

[[composites]]
name = "address"
fields = [
    { name = "street", pg_type = "text", nullable = false },
    { name = "city",   pg_type = "text", nullable = false },
]
```

### 6.3 Per-query cache file

```json
{
  "version": 1,
  "sql_hash": "a3f7c2e9b1d4…",
  "sql_normalized": "SELECT id, email FROM users WHERE id = $1",
  "source_locations": [
    {"file": "src/handlers/user.rs", "line": 42}
  ],
  "params": [
    {"index": 1, "pg_type": "uuid", "oid": 2950}
  ],
  "columns": [
    {
      "name": "id", "pg_type": "uuid", "oid": 2950, "nullable": false,
      "origin": {"table": "users", "column": "id"}
    },
    {
      "name": "email", "pg_type": "text", "oid": 25, "nullable": false,
      "origin": {"table": "users", "column": "email"}
    }
  ],
  "query_kind": "Select",
  "has_returning": false
}
```

### 6.4 SQL normalization (for deterministic hashing)

1. Strip line/block comments (`--`, `/* … */`).
2. Collapse whitespace runs to a single space.
3. Trim leading/trailing whitespace.
4. Preserve string literal contents verbatim (do not touch `'…'` bodies).
5. Preserve identifier case (PG keywords are case-insensitive but identifiers may not be).

Formatting refactors do not invalidate the cache.

### 6.5 Versioning

- `.sentinel/.version` holds the current format version.
- Macro fails compilation if `version > supported` and advises upgrading `sntl-macros`.
- If `version < supported`, `sntl prepare` migrates the cache forward automatically.

---

## 7. Nullability Inference Engine

### 7.1 Algorithm (runs inside macro during Stage 3)

1. Parse SQL with sqlparser-rs into an AST.
2. Build a scope map from the FROM clause: alias → table.
3. Walk JOINs, recording the join type for every table in scope.
4. For each SELECT output expression, resolve its column origin and apply rules.

### 7.2 JOIN propagation

| Join type | Left preserves | Right preserves |
|-----------|----------------|-----------------|
| `INNER JOIN` | yes | yes |
| `LEFT JOIN` | yes | forced nullable |
| `RIGHT JOIN` | forced nullable | yes |
| `FULL JOIN` | forced nullable | forced nullable |
| `CROSS JOIN` | yes | yes |

### 7.3 Expression rules

| Expression | Output nullable |
|------------|-----------------|
| plain column reference | per schema + JOIN |
| `COALESCE(a, b, …)` | non-null if any arg non-null |
| `NULLIF(a, b)` | always nullable |
| `CASE … WHEN … ELSE …` | nullable if any branch nullable, or if ELSE absent |
| `col IS [NOT] NULL` | non-null boolean |
| `count(*)`, `count(col)` | non-null |
| `sum`, `avg`, `min`, `max` | nullable (empty group → NULL) |
| arithmetic / string ops | nullable if any operand nullable |
| `ROW_NUMBER() OVER …` | non-null |
| `LAG(col)`, `LEAD(col)` | nullable (boundary rows) |
| scalar subquery in SELECT | nullable (zero rows → NULL) |
| `EXISTS(…)` | non-null boolean |
| literal `NULL` | nullable |
| literal non-null (numeric, string) | non-null |

### 7.4 Unknown expressions

If an expression is not covered (user-defined function, opaque contrib function), inference defaults to nullable and emits a warning suggesting `non_null = [col]` override. When `macros.strict_nullable = false` (opt-in), unknowns default to non-null instead; this is off by default to stay safe.

### 7.5 Parameter type inference

1. Macro generates `PREPARE _s AS <user_sql>` against the live DB during `sntl prepare`.
2. Reads `pg_prepared_statements.parameter_types` for the OID list.
3. Resolves each OID through the sentinel-driver type registry.
4. The Rust argument's type must match (or be convertible via `Into`); otherwise compile error with fix hint.

### 7.6 Confidence tiers

- **Tier 1** — direct column reference + schema match → high confidence.
- **Tier 2** — COALESCE/CASE with deterministic branches → medium.
- **Tier 3** — UDF / unknown expression → low, surfaced with `help: consider … override`.

---

## 8. Target-Type Integration

`sntl::query_as!(T, …)` compile-time dispatch:

- **`T: Model`** — strict validation against `Model::COLUMNS`. Column names, PG types, and nullability must match. Missing columns from the SELECT list are a compile error. Relation fields are ignored (they are loaded separately via `.Include(...)`, not by SELECT).
- **`T: Partial<Parent>`** — validates against the partial column subset. Extra SELECT columns not in the partial are a compile error.
- **`T: FromRow`** — duck-type: for every struct field, there must be an output column of matching name and compatible type. Extra output columns are silently ignored. This is the escape hatch for ad-hoc shapes.

`#[derive(FromRow)]` is added to `sntl-macros`. It expands to a `FromRow` trait implementation that reads columns by name with typed getters.

---

## 9. CLI

### 9.1 v0.2 (MVP)

- `sntl prepare [--check] [--workspace <path>]` — scan workspace for `sntl::query*!` calls, prepare each against the DB, write `.sentinel/queries/*.json`, refresh `schema.toml`. `--check` exits non-zero if cache is stale, writes nothing.
- `sntl check` — validate `.sentinel/` consistency: version compatible, schema fresh (configurable threshold), no orphaned cache files.
- `sntl doctor` — diagnostic checklist covering `sentinel.toml`, DB reachability, PG version, cache freshness, schema drift, pending migrations, outdated `sntl-macros`. Each failure includes a one-line remediation command.

Example `sntl doctor` output:

```
✓ sentinel.toml found at /project/sentinel.toml
✓ Database connection OK (postgres://localhost:5432/myapp_dev)
✓ PostgreSQL 16.2 (supported: 13+)
✗ Cache outdated (last prepare: 5 days ago)
   → run: sntl prepare
✗ Schema drift detected: column users.deleted_at changed type
   → run: sntl prepare --force
⚠ 2 migrations pending in migrations/ but not applied
   → run: sntl migrate run
```

### 9.2 v0.3 (migrations)

- `sntl migrate add <name> [--sql]`
- `sntl migrate run` (wrapped in `PgAdvisoryLockGuard` so multi-instance deployments are race-safe by default, unlike sqlx-cli which requires a timeout flag)
- `sntl migrate revert`
- `sntl migrate status`

### 9.3 v0.4 (Prisma-beat)

- `sntl introspect [--out schema.toml]` — reverse-introspect DB into `schema.toml`.
- `sntl migrate diff` — compare `schema.toml` to DB, print pending changes.
- `sntl migrate dev` — diff → add migration → run, in one step.
- `sntl generate [--models]` — generate `#[derive(Model)]` Rust structs from `schema.toml`.
- `sntl lsp` — language server for inline `query!` validation, hover types, jump-to-schema, code actions.
- `sntl studio` — optional GUI (stretch).

### 9.4 Watch mode

```
sntl prepare --watch
```

Watches `src/**/*.rs` and `queries/**/*.sql`; on change, re-parses affected files and updates `.sentinel/` incrementally. Designed to coexist with `cargo watch -x check`.

### 9.5 Output style

Uses indicatif + colored output:

```
sntl prepare
  ⠋ Scanning workspace… found 47 queries
  ⠸ Preparing queries [████████▒▒] 38/47 (81%)
  ✓ All queries cached
  ✓ Schema snapshot updated (14 tables, 3 enums)

Done in 2.4s
```

---

## 10. Error Handling

Every compile error carries **what, where, why, fix** — with at least three fix suggestions: a Rust-side change, an SQL-side change, and an escape hatch.

### Example 1 — column mismatch

```
error: query result column `email_addr` does not match any field in `User`
  ┌─ src/handlers/user.rs:42:5
  │
42│     sntl::query_as!(User, "SELECT id, email_addr FROM users WHERE id = $1", id)
  │     ^^^^^^^^^^^^^^^^^^^^^ column `email_addr` not in User struct

note: `User` has fields: id, email, created_at
help: did you mean `email`? use `SELECT id, email AS email_addr` to alias
    = or: add `email_addr: String` field to User
    = or: use `sntl::query!` (anonymous record) if User shouldn't change
```

### Example 2 — nullable mismatch

```
error: type mismatch — column `users.deleted_at` is nullable in schema
  ┌─ src/handlers/user.rs:15:9
  │
15│     deleted_at: chrono::DateTime<Utc>,
  │                 ^^^^^^^^^^^^^^^^^^^^^^ expected Option<DateTime<Utc>>

note: schema snapshot .sentinel/schema.toml:42 declares `deleted_at` as nullable
help: change field type to `Option<chrono::DateTime<Utc>>`
    = or: add `non_null = [deleted_at]` override if guaranteed non-null here
    = or: use COALESCE in SQL: `COALESCE(deleted_at, '1970-01-01') AS deleted_at`
```

### Example 3 — offline cache miss

```
error: query not found in cache (.sentinel/queries/a3f7c2e9b1d4.json)
  ┌─ src/handlers/user.rs:42:5
  │
42│     sntl::query!("SELECT * FROM new_table WHERE id = $1", id)
  │     ^^^^^^^^^^^^ query not prepared

note: SENTINEL_OFFLINE=true — cannot connect for live validation
help: run `sntl prepare` with DB connection, then commit .sentinel/
    = or: temporarily use `sntl::query_unchecked!` (validation skipped)
```

Implementation notes:

- Use `proc-macro-error2` for rich spans.
- Levenshtein distance for did-you-mean suggestions.
- Every error code (`E0001`, `E0002`, …) links to `https://docs.rs/sntl/latest/sntl/errors/EXXXX`.

---

## 11. Testing

- **Macro expansion** — `sntl-macros/tests/expand/` using `trybuild`, with fixture `.sentinel/` directories per case. Every documented error has a failing `trybuild` case.
- **Schema analyzer** — `sntl-schema/tests/` with nullability rule tables and property-based tests (`proptest`) generating random SELECT shapes to verify invariants.
- **Runtime integration** — `sntl/tests/macro_query_test.rs` hitting live PG via the existing `docker-compose up -d` harness. Covers every query macro × every target-type path × fetch method.
- **CLI** — `sntl-cli/tests/` with `assert_cmd` + tempdir, running `prepare → check → doctor` end-to-end.
- **Coverage target** — ≥ 90% on macro expansion paths; 100% on nullability rule table.

---

## 12. Performance

### 12.1 Compile-time targets vs sqlx baseline

Baseline figures below are rough estimates on a representative mid-size project; exact numbers come from the benchmark suite (§12.3) and must be verified before release. These targets define what would make the feature a clear win, not a guaranteed outcome.

| Scenario | sqlx (est.) | Sentinel target |
|----------|-------------|-----------------|
| Cold build, 100 queries, no cache | ~45s | ≤ 30s (33% faster) |
| Warm build, cache hit | ~2s | ≤ 1s (50% faster) |
| Incremental, 1 query changed | ~3s | ≤ 500ms (6× faster) |

Sources of speedup:

- One file per query → only changed queries are re-parsed.
- No monolithic manifest → no global parse cost.
- Schema parse is shared across the compilation unit (loaded once).

### 12.2 Runtime targets

| Scenario | sqlx | Sentinel |
|----------|------|----------|
| Single prepared query | baseline | identical |
| 3 pipelined queries | 3 round-trips | 1 round-trip (3× faster) |
| Bulk insert 10k rows | INSERT batch | COPY protocol (10–50× faster) |
| Cursor over large result | manual | `CursorQuery` built-in, memory-bounded |

Sources of speedup:

- `query_typed()` skips the Prepare round-trip when OIDs are cached.
- Two-tier statement cache (HashMap + LRU-256) in sentinel-driver reaches ~99% hit rate.
- Binary wire format + zero-copy `bytes::Bytes` decode.
- COPY protocol exposed via macro (`Model::copy_in`, separate RFC).

### 12.3 Benchmark suite (ships with v0.2)

```
sntl/benches/
├── macro_expand.rs        # criterion: macro expansion time
├── query_single.rs        # vs sqlx::query! single query
├── query_pipeline.rs      # vs sqlx sequential (pipeline advantage)
├── bulk_insert.rs         # vs sqlx INSERT batch (COPY advantage)
└── schema_analyze.rs      # nullability inference throughput
```

Results published in README and release blog post.

---

## 13. Observability

- Every expanded query emits a `tracing::span!` named `sntl::query` with attributes `hash`, `file`, `line`, `query_kind`.
- Users enable with `RUST_LOG=sntl=debug`.
- Integrates with existing `QueryMetrics` from sentinel-driver (elapsed, rows affected, cache hit tier).

---

## 14. Migration Story (sqlx → Sentinel)

A user coming from sqlx should reach a working build with minimal friction:

1. Add `sntl` and `sntl-macros` to `Cargo.toml`; remove `sqlx` dependency.
2. Rename `sqlx::query!` → `sntl::query!` (call shape is identical).
3. Delete `.sqlx/` (cache format differs) and run `sntl prepare` to generate `.sentinel/`.
4. Replace `#[derive(sqlx::FromRow)]` with `#[derive(sntl::FromRow)]`.
5. `cargo build` — compile errors point to exact remediation.

A dedicated migration cookbook will ship with v0.2 docs.

---

## 15. Open Questions

- **Parameter type coercion**: should we allow `i32 → i64` coercion via `Into`, or require exact match? Leaning toward exact match with explicit `cast as i64` advice for clarity.
- **Dynamic IN clauses**: `WHERE id = ANY($1)` with `Vec<T>` is supported; full `WHERE id IN (…)` with variable arity is not. Decide whether to expose a `sntl::QueryBuilder` that composes with `query!` for the dynamic tail.
- **Multi-schema support**: `schema.toml` currently assumes `public`; multi-schema projects (cross-schema FKs) need a scoping rule. Probably include `schema` field per table and qualify origins with `{schema}.{table}`.
- **Async transactions inside pipelines**: can `query_pipeline!` run inside a `Transaction`? Semantics need verification against sentinel-driver pipeline-mode contract.

These are tracked for resolution before the implementation plan is finalized.

---

## 16. Rollout Plan

- **v0.2** — macro surface complete, `sntl-cli` has `prepare/check/doctor`, benchmarks published.
- **v0.3** — migration CLI (`sntl migrate …`) with advisory-lock safety.
- **v0.4** — introspection, codegen, LSP. At this point Sentinel offers a DX envelope that exceeds Prisma's today.
- **v0.5+** — `sntl studio` GUI, bidirectional schema validation, cross-cloud runtime adapters.
