# Cluster A: Array Element Nullability + Tuple FromRow — Design

> **Status:** approved by user, ready for implementation plan
> **Scope:** macro-side improvements that close the array nullability gap
> versus sqlx and unlock plain tuple `FromRow`. Driver-side companion PR
> in `sentinel-driver` is required first.

## 1. Goals + Scope

### Ship in this design

1. **Auto-emit `Vec<Option<T>>`** — when `query!()`/`query_as!()` returns an
   array column, the generated record field is `Vec<Option<T>>` by default
   (conservative element nullability). Resolution is schema-driven via a
   new `ColumnInfo.element_type` field that survives the existing cache
   format without a version bump.

2. **`non_null_elements = [col1, col2]` override** — opt-out keyword in
   the macro argument list that drops the per-element `Option<T>` wrapper
   when the user is certain elements cannot be NULL.

3. **Plain tuple `FromRow`** — blanket impl for arities 1 through 16 in
   `sntl::__macro_support`, allowing `query_as!((i32, String), …)`
   without a wrapper struct. Mirrors sqlx's `from_row.rs` `macro_rules!`
   block.

4. **Re-export hstore/ltree/cube** — separate trivial PR (PR-1, no design
   needed). Adds `pub use driver::types::hstore;` etc. so the existing
   driver coverage of those extensions is reachable through `sntl`.

### Companion PR in sentinel-driver

Required before sentinel can ship:

5. **`Decode for Vec<Option<T>>`** — blanket impl that handles per-element
   NULL via the existing `-1` length sentinel. Without this, the macro
   would emit valid Rust that panics at runtime on NULL elements.

6. **`PgHasArrayType` (or equivalent)** — `Vec<Option<T>>` advertises the
   same Postgres array type info as `Vec<T>`.

### Out of scope

- Multi-dimensional arrays (`Vec<Vec<T>>`) — postponed to driver Phase 2 P1
- User-defined composite types — v0.3 candidate
- Cache format version bump — `element_type` is added as
  `#[serde(default)] Option<…>` so old caches deserialize as `None`
- Lifting `query_typed_*` into `GenericClient` trait — separate driver work
- Pool integration in macros — depends on the trait lift above

### Success criteria

| # | Statement |
|---|-----------|
| 1 | Driver: `cargo test -p sentinel-driver` covers `Vec<Option<T>>` round-trip with mixed NULL elements |
| 2 | `query!("SELECT tags FROM users")` (where `tags text[]`) emits a record field of type `Vec<Option<String>>` |
| 3 | `query!("SELECT tags FROM users", non_null_elements = [tags])` emits `Vec<String>` |
| 4 | `query_as!((i32, String), "SELECT id, email FROM users WHERE id = $1", 1)` compiles and returns the tuple directly |
| 5 | Old `.sentinel/queries/<hash>.json` files (no `element_type`) still deserialize and compile (backward compat) |
| 6 | All trybuild fixtures pass; live-PG round-trip test extends with array case |
| 7 | Misuse (`non_null_elements` referencing a non-array column, tuple arity mismatch) produces a `proc_macro_error2::abort!` with a help hint, not a runtime crash |

---

## 2. Architecture + Data Flow

### Layer responsibilities

```
┌─────────────────────────────────────────────────────────────────┐
│ User code: sntl::query!("SELECT tags FROM users")               │
│                                ↓                                │
├─────────────────────────────────────────────────────────────────┤
│ sntl-macros (proc-macro time)                                   │
│   1. lookup .sentinel/queries/<hash>.json                       │
│   2. for each ColumnInfo:                                       │
│      • element_type.is_some()?       → array path               │
│      • col in non_null_elements?     → Vec<T> else Vec<Option<T>> │
│   3. emit struct field with proper Rust type                    │
├─────────────────────────────────────────────────────────────────┤
│ sntl-schema (compile-time data)                                 │
│   ColumnInfo {                                                  │
│     name, pg_type, oid, nullable, origin,                       │
│     element_type: Option<ElementTypeRef>,  // NEW (additive)    │
│   }                                                             │
│   ElementTypeRef { pg_type: String, oid: u32 }                  │
├─────────────────────────────────────────────────────────────────┤
│ sntl-cli (sntl prepare, runtime)                                │
│   1. Connection::prepare(sql) → Statement (existing)            │
│   2. NEW: for each output column,                               │
│      look up element type via SELECT typelem, typname FROM pg_type │
│   3. write ColumnInfo with element_type populated               │
├─────────────────────────────────────────────────────────────────┤
│ sentinel-driver (runtime, separate PR)                          │
│   • blanket Decode for Vec<Option<T>> via per-element -1 length │
│   • PgHasArrayType for Option<T> delegates to T                 │
└─────────────────────────────────────────────────────────────────┘
```

### Happy-path trace — `tags text[]` column with a NULL element

1. **`sntl prepare` time:**
   - `Connection::prepare("SELECT tags FROM users")` returns a `Statement`
     whose first column has `oid = 1009` (Postgres `_text`)
   - Element-type lookup: `SELECT typelem, typname FROM pg_type WHERE oid = 1009`
     returns `(25, "text")`
   - Write
     `ColumnInfo { name: "tags", pg_type: "_text", oid: 1009, element_type: Some(ElementTypeRef { pg_type: "text", oid: 25 }), … }`
     into `.sentinel/queries/<hash>.json`

2. **Compile time (`query!` expansion):**
   - Macro reads cache, sees `element_type.is_some()` and that "tags" is not
     listed in `non_null_elements` → emits `Vec<Option<String>>`
   - Generated struct field: `pub tags: Vec<Option<String>>`
   - Generated getter: `tags: row.try_get_by_name::<Vec<Option<String>>>("tags")?`

3. **Runtime:**
   - Driver's `Vec<Option<String>>` decode walks the binary array format
   - For each element: `length == -1` pushes `None`; otherwise calls
     `Option<String>::decode(slice)` and pushes `Some(decoded)`

### Component-interface change summary

| Component | Public surface change | Internal change |
|---|---|---|
| `sentinel_driver::Row::try_get_by_name` | none | none — relies on existing `T: FromSql` plus the new `Vec<Option<T>>` impl |
| `sntl_schema::cache::ColumnInfo` | additive: `#[serde(default)] element_type: Option<ElementTypeRef>` | none |
| `sntl_schema::introspect::prepare_query` | none — same fn signature | extra `pg_type` lookup when `oid` belongs to the array set |
| `sntl_macros::query::codegen::rust_type_for_column` | none — same fn signature | branch on `element_type.is_some()`, then on override membership |
| `sntl_macros::query::args::QueryArgs` | additive: optional `non_null_elements: Vec<Ident>` |
| `sntl::__macro_support` | new pub: `FromRow for (T1, …, Tn)` arities 1–16 | uses existing `FromRow` trait shape |

### Out-of-band concerns

- **Backward compat:** old cache (no `element_type`) → field defaults to
  `None` → macro takes the existing scalar path
- **Forward compat:** old binary reading new cache → `serde_json` ignores
  unknown fields by default → still works for non-array queries; array
  columns lose nullability inference (degraded, not broken)
- **Driver coupling:** `sntl/Cargo.toml` will pin sentinel-driver to a new
  minimum version once the companion PR ships

---

## 3. Override syntax + nullability rules

### Syntax extension

`QueryArgs` parser in `sntl-macros/src/query/args.rs` currently accepts
`nullable=[…]` and `non_null=[…]`. Add **one** keyword:

```rust
sntl::query!(
    "SELECT id, tags FROM users WHERE id = $1",
    user_id,
    non_null_elements = [tags]   // NEW — element-level override
)
```

No `nullable_elements` keyword. Reason: the design defaults array
elements to nullable, so requesting nullable is a no-op.

### Type emission matrix

Two layers of `Option`: outer (the column itself can be NULL) × inner
(the element can be NULL).

| Column NULL? | Element NULL (default = yes) | Override applied | Emitted Rust type |
|---|---|---|---|
| no | yes | — (default) | `Vec<Option<T>>` |
| no | yes | `non_null_elements = [col]` | `Vec<T>` |
| yes | yes | — | `Option<Vec<Option<T>>>` |
| yes | yes | `non_null = [col]` | `Vec<Option<T>>` |
| yes | yes | `non_null_elements = [col]` | `Option<Vec<T>>` |
| yes | yes | both `non_null` + `non_null_elements` | `Vec<T>` |
| yes | yes | `nullable = [col]` (no-op) | `Option<Vec<Option<T>>>` |
| no | yes | `nullable = [col]` | `Option<Vec<Option<T>>>` |

### Override validation (proc-macro time)

- `non_null_elements = [foo]` where `foo` is not an array column →
  `compile_error!("non_null_elements references `foo` which is not an array column")`
- `non_null_elements = [bar]` where `bar` is not an output column →
  `compile_error!("override `bar` is not an output column")` (matches
  the existing `non_null` validation path)
- Validation lives in `sntl_schema::resolve::resolve_offline`. `ResolveInput`
  gains `overrides_non_null_elements: &'a [String]`.

### Plain tuple `FromRow`

Lives in `sntl/src/core/query/macro_support.rs`:

```rust
macro_rules! impl_from_row_tuple {
    ($($ty:ident at $idx:tt),+) => {
        impl<$($ty),+> FromRow for ($($ty,)+)
        where
            $($ty: ::sntl::driver::FromSql),+
        {
            fn from_row(row: &::sntl::driver::Row) -> Result<Self> {
                Ok(($(row.try_get::<$ty>($idx).map_err(|e| Error::Driver(e))?,)+))
            }
        }
    };
}

impl_from_row_tuple!(T0 at 0);
impl_from_row_tuple!(T0 at 0, T1 at 1);
// … through arity 16
```

Notes:

- Same arity range as sqlx (1–16).
- Uses positional `try_get(idx: usize)`. Element 0 = first SELECT column.
- Wraps `driver::Error` through the existing `sntl::Error::Driver(#[from])`.

### `query_as!` macro changes

- Macro emits `_assert_from_row::<#target>()` followed by
  `QueryExecution::<#target>::new(…)`. Tuples work for free because the
  blanket impl satisfies the bound.
- **Validation:** when `#target` is a tuple, check
  `tuple_arity == resolved.columns.len()`. Mismatch → `abort!` with a
  help hint that lists actual SELECT columns.

### Field/column ordering for tuples

- sqlx convention: positional index, tuple element 0 = first SELECT column.
- Sentinel adopts the same convention. Implication: changing the SELECT
  column order changes tuple element types — accepted trade-off because
  this is the ad-hoc-query use case.

---

## 4. Error handling, diagnostics, and testing

### Error handling

**Driver layer (new in sentinel-driver PR):**

- `Vec<Option<T>>` decode reuses `Option<T>::from_sql_nullable(None)` →
  returns `Ok(None)` on per-element NULL — never panics.
- Invalid array header (`ndim ≠ 1`, lower bound ≠ 1) → existing
  `driver::Error::Decode("…")` (parity with sqlx).
- `Vec<T>` decode (no `Option`) on an array containing NULL → returns
  `Err(driver::Error::Decode("array element NULL but T not Option"))` —
  clear message instead of a panic.

**Schema layer (sntl-schema):**

- New: `Error::Config("override `non_null_elements` references non-array column `<col>`")`
  (reuse the existing `Config(String)` variant).
- Existing `Error::UnknownColumn` reused when an override references a column
  not in the output set.

**Macro layer:**

- All compile-time errors via `proc_macro_error2::abort!` with `help =`
  hints; no panics in the proc-macro path.

### Diagnostic examples

```
error: non_null_elements references `tags` which is not an array column
  --> src/main.rs:42:25
   |
42 |     non_null_elements = [tags]
   |                          ^^^^
   |
   = help: remove the override, or run `sntl prepare` if the column type changed
   = help: array columns have non-empty element_type in .sentinel/queries/<hash>.json
```

```
error: query_as! tuple arity mismatch
  --> src/main.rs:42:5
   |
42 |     sntl::query_as!((i32, String), "SELECT id, name, email FROM users")
   |                     ^^^^^^^^^^^^^
   |
   = help: tuple expects 2 columns; SELECT returns 3 (id, name, email)
   = help: extend to (i32, String, String) or trim the SELECT list
```

### Testing strategy

| Layer | Test type | What |
|---|---|---|
| driver | unit | synthesized binary: all-NULL array, mixed, empty, single-NULL |
| driver | live-PG | `Vec<Option<i32>>` round-trip with NULL element; `Vec<i32>` decode of a NULL-bearing array → assert `Err`, not panic |
| sntl-schema | unit | old cache (no `element_type`) deserialises with `None` (backward compat) |
| sntl-schema | unit | `resolve_offline` with `overrides_non_null_elements` populated mutates nullability correctly |
| sntl-schema | unit | validation: `non_null_elements` pointing at non-array → `Err(Config(…))` |
| sntl-cli | live-PG | `sntl prepare` writes cache JSON with `element_type` populated for array columns |
| sntl-cli | live-PG | array column whose element type is missing from `pg_type` (rare custom domain) → `element_type` stays `None`, no crash |
| sntl-macros | trybuild pass | `query!("SELECT tags FROM …")` compiles, generated struct has `Vec<Option<String>>` field |
| sntl-macros | trybuild pass | with `non_null_elements = [tags]` override → `Vec<String>` |
| sntl-macros | trybuild pass | `query_as!((i32, String), …)` compiles |
| sntl-macros | trybuild compile_fail | `non_null_elements = [non_array_col]` → diagnostic snapshot |
| sntl-macros | trybuild compile_fail | tuple arity mismatch → diagnostic snapshot |
| sntl integration | live-PG | INSERT row with `tags = ARRAY['a', NULL, 'b']` → `query!` returns `Vec<Option<String>>`, asserts `[Some, None, Some]` |
| sntl integration | live-PG | `non_null_elements` override + all-non-null data → returns `Vec<String>` |
| sntl integration | live-PG | `non_null_elements` override + NULL element → runtime `Err(Driver(…))`, message identifies array column |

### Test fixture extensions

- `tests/integration/setup.sql` — add `tags text[]` column to the users
  table (hand-rolled because `sntl-migrate` does not exist yet).
- `.sentinel/queries/<new-hash>.json` × 2 — for the with/without-override
  test queries.
- `.sentinel/schema.toml` — extend the users table to include `tags`.
- driver: synthesized binary array bytes in `sentinel-driver/tests/array_decode.rs`.

### Coverage plan

- driver PR: aim ≥95 % line coverage for the new `Vec<Option<T>>` decode
  path (excluding error branches that need invalid-binary fixtures).
- sentinel PRs: maintain the existing CI exclude pattern; `sntl-schema/`
  and `macro_support.rs` remain excluded because they sit on the live-PG
  / runtime paths.
- The new `non_null_elements` override path in `resolve_offline` will be
  unit-covered.

---

## 5. Implementation cadence

Three PRs in this repo plus one driver PR up front:

| Order | Repo | PR | Notes |
|---|---|---|---|
| 1 | sentinel-driver | `Decode for Vec<Option<T>>` + `PgHasArrayType` for `Option<T>` + tests | Required prerequisite |
| 2 | sentinel | PR-1 warm-up: re-export hstore/ltree/cube via `sntl::types::*` | Trivial; does not depend on driver PR |
| 3 | sentinel | PR-2 array work: cache field, introspect, codegen, override, fixtures | Pins minimum driver version |
| 4 | sentinel | PR-3 tuple FromRow blanket impls + trybuild fixtures | Independent of PR-2 |

Checkpoint after each PR; do not start the next until tests + clippy +
fmt + cargo-deny are green.

---

## 6. Open items / verification before implementation

- Confirm `sentinel-driver`'s exact array OID set (anything ≥ 10000 is
  user-defined and needs runtime `pg_type` lookup, not a hardcoded table).
- Confirm `Connection::prepare` returns enough column metadata to drive
  the `pg_type` lookup, or whether `sntl prepare` must issue a separate
  query per array column.
- Decide whether the element-type lookup in `sntl prepare` should batch
  array OIDs into one `WHERE oid = ANY($1)` query rather than N queries.
  Default: batch.
