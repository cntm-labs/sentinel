# Cluster A: Array Element Nullability + Tuple FromRow — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Auto-emit `Vec<Option<T>>` for array columns and accept plain tuple `FromRow` targets — closing parity with sqlx without sacrificing Sentinel's compile-time-guard story.

**Architecture:** Driver gains `Vec<Option<T>>` decode (parity with sqlx). Sentinel adds an additive `element_type: Option<ElementTypeRef>` field to `ColumnInfo` (no cache version bump), populates it during `sntl prepare` via a single batched `pg_catalog.pg_type` query, and the `query!()` family branches on it to emit either `Vec<T>`, `Vec<Option<T>>`, or `Option<…>` wrappers based on existing column nullability plus a new `non_null_elements = […]` override. Plain tuple `FromRow` ships as a blanket `macro_rules!` impl in `sntl::__macro_support` covering arities 1–16.

**Tech Stack:** Rust 1.85 / edition 2024, syn 2, quote 1, proc-macro-error2, sqlparser 0.57, serde + serde_json (additive serde defaults for backward compat), sentinel-driver 1.0.x with the new `Decode<Vec<Option<T>>>` from Phase 0.

---

## Reference material

The design doc this plan implements is `docs/plans/2026-04-29-cluster-a-array-tuple-design.md`. Read §1 (goals + scope), §3 (override syntax + nullability matrix), and §4 (testing strategy) before starting. Every behavioural question resolves there.

The previous query-macro plan at `docs/plans/2026-04-20-sntl-query-macro-impl.md` set the conventions used here (TDD per task, file lists, expected commands + outputs, commit per task).

---

## File structure (master map)

This plan creates or modifies exactly the following paths.

### Phase 0 — sentinel-driver repo (`../sentinel-driver`, separate PR)

```
sentinel-driver/crates/sentinel-driver/
├── src/types/decode.rs           # MODIFY: split decode_array into nullable/non-nullable; add Vec<Option<T>> impls
└── tests/array_nullable_test.rs  # CREATE: synthesized binary + live-PG round-trip
```

### Phase 1 — sentinel re-export warm-up (`sntl/`, PR-1)

```
sntl/src/lib.rs                   # MODIFY: pub use driver::types::{hstore, ltree, cube}
sntl/tests/types_reexport_test.rs # CREATE: smoke test that the re-exports resolve
```

### Phase 2 — sentinel array work (`sntl-schema/`, `sntl-macros/`, `sntl-cli/`, `sntl/`, PR-2)

```
sntl-schema/src/cache.rs                       # MODIFY: add ElementTypeRef + ColumnInfo.element_type
sntl-schema/tests/cache_element_type_test.rs   # CREATE: backward compat + roundtrip
sntl-schema/src/resolve.rs                     # MODIFY: ResolveInput + overrides_non_null_elements
sntl-schema/tests/resolve_element_test.rs      # CREATE: override application + validation errors
sntl-schema/src/introspect.rs                  # MODIFY: batch pg_type query for element types
sntl-macros/src/query/args.rs                  # MODIFY: parse non_null_elements keyword
sntl-macros/src/query/codegen.rs               # MODIFY: rust_type_for_column branches on element_type
sntl-macros/src/query/anonymous.rs             # MODIFY: forward non_null_elements
sntl-macros/src/query/typed.rs                 # MODIFY: forward non_null_elements
sntl-macros/src/query/file.rs                  # MODIFY: forward non_null_elements
sntl-macros/tests/expand/query/array_basic.rs            # CREATE: trybuild pass — Vec<Option<T>>
sntl-macros/tests/expand/query/array_non_null.rs         # CREATE: trybuild pass — non_null_elements override
sntl-macros/tests/expand/query/non_null_elements_bad.rs  # CREATE: trybuild compile_fail — non-array column
sntl-macros/tests/expand/query/non_null_elements_bad.stderr  # CREATE: diagnostic snapshot
.sentinel/schema.toml                          # MODIFY: add tags text[] to users
.sentinel/queries/<hash>.json                  # CREATE: cache entries for the new test queries (× 2)
tests/integration/setup.sql                    # MODIFY: ALTER TABLE users ADD COLUMN tags text[]
sntl/tests/macro_array_test.rs                 # CREATE: live-PG round-trip with NULL elements
```

### Phase 3 — sentinel tuple FromRow (`sntl/`, `sntl-macros/`, PR-3)

```
sntl/src/core/query/macro_support.rs                # MODIFY: macro_rules! impl_from_row_tuple + 16 invocations
sntl/tests/macro_tuple_from_row_test.rs             # CREATE: live-PG round-trip with tuple target
sntl-macros/src/query/typed.rs                      # MODIFY: tuple arity validation
sntl-macros/tests/expand/query/tuple_basic.rs       # CREATE: trybuild pass — query_as!((i32, String), …)
sntl-macros/tests/expand/query/tuple_arity_bad.rs   # CREATE: trybuild compile_fail
sntl-macros/tests/expand/query/tuple_arity_bad.stderr   # CREATE: diagnostic snapshot
.sentinel/queries/<hash>.json                       # CREATE: cache for new tuple test query
```

---

## Phase 0 — sentinel-driver companion PR

> All Phase 0 work happens in `../sentinel-driver`, on a new branch in that repo. After it merges and a 1.0.x patch release ships, sentinel `cargo update` picks it up automatically.

### Task 1: Make `decode_array` accept a nullability mode

**Files:**
- Modify: `../sentinel-driver/crates/sentinel-driver/src/types/decode.rs:234-293`

- [ ] **Step 1: Replace `decode_array` with a generic that returns `Vec<Either<T, Null>>` semantics via two callers**

Replace lines 234–293 of `src/types/decode.rs` with:

```rust
fn decode_array_inner<T, F>(buf: &[u8], expected_elem_oid: Oid, decode_elem: F) -> Result<Vec<T>>
where
    F: Fn(Option<&[u8]>) -> Result<T>,
{
    if buf.len() < 12 {
        return Err(Error::Decode("array: header too short".into()));
    }

    let ndim = read_i32(buf, 0);
    // has_null at buf[4..8] is informational only — we honour the per-element -1 sentinel
    let elem_oid = read_u32(buf, 8);

    if ndim == 0 {
        return Ok(Vec::new());
    }

    if ndim != 1 {
        return Err(Error::Decode(format!(
            "array: multi-dimensional arrays not supported (ndim={ndim})"
        )));
    }

    if elem_oid != expected_elem_oid.0 {
        return Err(Error::Decode(format!(
            "array: expected element OID {}, got {elem_oid}",
            expected_elem_oid.0
        )));
    }

    if buf.len() < 20 {
        return Err(Error::Decode("array: dimension header too short".into()));
    }

    let dim_len = read_i32(buf, 12) as usize;

    let mut offset = 20;
    let mut result = Vec::with_capacity(dim_len);

    for _ in 0..dim_len {
        if offset + 4 > buf.len() {
            return Err(Error::Decode("array: unexpected end of data".into()));
        }

        let elem_len = read_i32(buf, offset);
        offset += 4;

        if elem_len < 0 {
            // Per-element NULL sentinel
            result.push(decode_elem(None)?);
        } else {
            let elem_len = elem_len as usize;
            if offset + elem_len > buf.len() {
                return Err(Error::Decode("array: element data truncated".into()));
            }
            result.push(decode_elem(Some(&buf[offset..offset + elem_len]))?);
            offset += elem_len;
        }
    }

    Ok(result)
}

fn decode_array<T: FromSql>(buf: &[u8], expected_elem_oid: Oid) -> Result<Vec<T>> {
    decode_array_inner(buf, expected_elem_oid, |opt| match opt {
        Some(bytes) => T::from_sql(bytes),
        None => Err(Error::Decode("array: NULL elements not supported (use Vec<Option<T>>)".into())),
    })
}

fn decode_array_nullable<T: FromSql>(buf: &[u8], expected_elem_oid: Oid) -> Result<Vec<Option<T>>> {
    decode_array_inner(buf, expected_elem_oid, |opt| match opt {
        Some(bytes) => T::from_sql(bytes).map(Some),
        None => Ok(None),
    })
}
```

- [ ] **Step 2: Verify existing tests still pass**

Run: `cargo test -p sentinel-driver --lib types::decode 2>&1 | tail -10`
Expected: all existing array-decode tests pass — the public surface of `decode_array` is unchanged.

- [ ] **Step 3: Commit**

```bash
git add crates/sentinel-driver/src/types/decode.rs
git commit -m "refactor(types): split decode_array into nullable/non-nullable helpers"
```

---

### Task 2: Add `Vec<Option<T>>` `FromSql` impls

**Files:**
- Modify: `../sentinel-driver/crates/sentinel-driver/src/types/decode.rs:295-end`

- [ ] **Step 1: Extend the macro to also emit a nullable-element impl**

Replace the `macro_rules! impl_array_from_sql { … }` definition with:

```rust
/// Implements `FromSql` for `Vec<T>` and `Vec<Option<T>>` for the given element type.
macro_rules! impl_array_from_sql {
    ($elem_ty:ty, $array_oid:expr, $elem_oid:expr) => {
        impl FromSql for Vec<$elem_ty> {
            fn oid() -> Oid {
                $array_oid
            }

            fn from_sql(buf: &[u8]) -> Result<Self> {
                decode_array::<$elem_ty>(buf, $elem_oid)
            }
        }

        impl FromSql for Vec<Option<$elem_ty>> {
            fn oid() -> Oid {
                $array_oid
            }

            fn from_sql(buf: &[u8]) -> Result<Self> {
                decode_array_nullable::<$elem_ty>(buf, $elem_oid)
            }
        }
    };
}
```

The 24+ existing `impl_array_from_sql!` invocations below (`bool`, `i16`, `i32`, … `NaiveTime`, `PgPoint`, etc.) now generate both impls — no change to those lines.

- [ ] **Step 2: Verify both impls compile**

Run: `cargo check -p sentinel-driver --all-features 2>&1 | tail -5`
Expected: clean — no overlapping-impl errors.

- [ ] **Step 3: Commit**

```bash
git add crates/sentinel-driver/src/types/decode.rs
git commit -m "feat(types): add FromSql for Vec<Option<T>> across all array element types"
```

---

### Task 3: Tests for `Vec<Option<T>>` round-trip

**Files:**
- Create: `../sentinel-driver/crates/sentinel-driver/tests/array_nullable_test.rs`

- [ ] **Step 1: Write the synthesized-binary unit tests**

Create `tests/array_nullable_test.rs`:

```rust
//! Round-trip and binary-decode tests for Vec<Option<T>> array support.
//! Live-PG tests skip silently when DATABASE_URL is unset.

use sentinel_driver::types::{FromSql, Oid};

/// Build a binary array body (without the BIND header) for testing decode paths.
/// Layout: [ndim:i32][has_null:i32][elem_oid:u32][dim_len:i32][lbound:i32]
///         [elem_len:i32][elem_bytes]…  (elem_len = -1 for NULL)
fn build_array_bytes(elem_oid: u32, elements: &[Option<&[u8]>]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&1i32.to_be_bytes()); // ndim
    let has_null = elements.iter().any(|e| e.is_none()) as i32;
    buf.extend_from_slice(&has_null.to_be_bytes());
    buf.extend_from_slice(&elem_oid.to_be_bytes());
    buf.extend_from_slice(&(elements.len() as i32).to_be_bytes()); // dim_len
    buf.extend_from_slice(&1i32.to_be_bytes()); // lbound
    for e in elements {
        match e {
            Some(bytes) => {
                buf.extend_from_slice(&(bytes.len() as i32).to_be_bytes());
                buf.extend_from_slice(bytes);
            }
            None => buf.extend_from_slice(&(-1i32).to_be_bytes()),
        }
    }
    buf
}

#[test]
fn decode_vec_option_int4_with_nulls() {
    let bytes = build_array_bytes(
        23, // INT4
        &[Some(&1i32.to_be_bytes()), None, Some(&3i32.to_be_bytes())],
    );
    let v: Vec<Option<i32>> = FromSql::from_sql(&bytes).unwrap();
    assert_eq!(v, vec![Some(1), None, Some(3)]);
}

#[test]
fn decode_vec_option_text_all_null() {
    let bytes = build_array_bytes(25 /* TEXT */, &[None, None]);
    let v: Vec<Option<String>> = FromSql::from_sql(&bytes).unwrap();
    assert_eq!(v, vec![None, None]);
}

#[test]
fn decode_vec_option_empty() {
    // ndim=0 path
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0i32.to_be_bytes()); // ndim
    bytes.extend_from_slice(&0i32.to_be_bytes()); // has_null
    bytes.extend_from_slice(&25u32.to_be_bytes()); // elem_oid
    bytes.extend_from_slice(&[0u8; 4]); // padding (decoder reads 12 bytes header but exits early on ndim==0)
    let v: Vec<Option<String>> = FromSql::from_sql(&bytes).unwrap();
    assert_eq!(v, Vec::<Option<String>>::new());
}

#[test]
fn decode_vec_int4_rejects_null_element() {
    let bytes = build_array_bytes(23, &[Some(&1i32.to_be_bytes()), None]);
    let err = <Vec<i32> as FromSql>::from_sql(&bytes).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("NULL elements not supported"),
        "unexpected error: {msg}"
    );
}
```

- [ ] **Step 2: Add a live-PG round-trip test (gated on DATABASE_URL)**

Append to the same file:

```rust
#[macro_use]
mod test_helpers {
    macro_rules! require_pg {
        () => {
            match std::env::var("DATABASE_URL").ok() {
                Some(url) => url,
                None => return,
            }
        };
    }
}

#[tokio::test]
async fn vec_option_int4_roundtrip_live() {
    let url = require_pg!();
    let cfg = sentinel_driver::Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(cfg).await.unwrap();

    let row = conn
        .query_one("SELECT ARRAY[1, NULL, 3]::int4[] AS v", &[])
        .await
        .unwrap();
    let v: Vec<Option<i32>> = row.try_get(0).unwrap();
    assert_eq!(v, vec![Some(1), None, Some(3)]);
}

#[tokio::test]
async fn vec_int4_rejects_null_live() {
    let url = require_pg!();
    let cfg = sentinel_driver::Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(cfg).await.unwrap();

    let row = conn
        .query_one("SELECT ARRAY[1, NULL, 3]::int4[] AS v", &[])
        .await
        .unwrap();
    let err = row.try_get::<_, Vec<i32>>(0).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("NULL elements not supported"), "{msg}");
}
```

- [ ] **Step 3: Run unit tests**

Run: `cargo test -p sentinel-driver --test array_nullable_test 2>&1 | tail -10`
Expected: 4 unit tests pass; live-PG tests print "skipping" or pass depending on DATABASE_URL.

- [ ] **Step 4: Commit**

```bash
git add crates/sentinel-driver/tests/array_nullable_test.rs
git commit -m "test(types): synthesized + live-PG coverage for Vec<Option<T>> arrays"
```

---

### Task 4: Driver PR + release + version bump

- [ ] **Step 1: Open PR in sentinel-driver repo**

From the sentinel-driver worktree:

```bash
git push -u origin <branch-name>
gh pr create --title "feat(types): Vec<Option<T>> array decode for nullable elements" \
  --body "Adds FromSql for Vec<Option<T>> across all 24+ existing array element types, plus refactors decode_array into nullable/non-nullable helpers. Closes a parity gap with sqlx and unblocks sentinel macro work (Cluster A)."
```

- [ ] **Step 2: After merge, cut a 1.0.x patch release per the driver's release process**

Refer to the driver repo's release docs. Tag e.g. `v1.0.1`.

- [ ] **Step 3: Bump the dependency in this sentinel repo**

In `Cargo.toml` (workspace root):

```toml
sentinel-driver = "1.0.1"  # was "1.0.0"
```

Run: `cargo update -p sentinel-driver`
Verify: `cargo check --workspace` still passes.

- [ ] **Step 4: Commit the bump**

```bash
git add Cargo.toml
git commit -m "deps: bump sentinel-driver to 1.0.1 for Vec<Option<T>> support"
```

> **Phase 0 gate:** Do not start Phase 1 until Task 4 is committed and `cargo build` produces no errors.

---

## Phase 1 — sentinel re-export warm-up (PR-1)

### Task 5: Re-export hstore/ltree/cube

**Files:**
- Modify: `sntl/src/lib.rs`
- Create: `sntl/tests/types_reexport_test.rs`

- [ ] **Step 1: Locate the existing driver type modules**

Run: `ls /home/mrbt/Desktop/workspaces/orm/repositories/sentinel-driver/crates/sentinel-driver/src/types/ | grep -E '^(hstore|ltree|cube)'`
Expected: `hstore.rs`, plus either `ltree.rs` and `cube.rs` or evidence they live elsewhere. If `ltree.rs` or `cube.rs` is missing in driver, drop that name from this task (do not invent a re-export for code that isn't there).

- [ ] **Step 2: Add re-exports**

Append to `sntl/src/lib.rs` (under the existing `pub use driver::…` block):

```rust
/// PostgreSQL extension types re-exported from the driver.
pub mod types {
    pub use driver::types::hstore;
    // Add `ltree` / `cube` only if Step 1 confirmed the driver has them.
}
```

- [ ] **Step 3: Smoke-test the re-export resolves**

Create `sntl/tests/types_reexport_test.rs`:

```rust
//! Compile-only test that asserts the re-exports resolve.

#[test]
fn hstore_module_resolves() {
    // Just touching the path is enough; if it didn't resolve, this file
    // wouldn't compile.
    let _ = std::any::type_name::<sntl::types::hstore::PgHstore>();
}
```

- [ ] **Step 4: Run**

Run: `cargo test -p sntl --test types_reexport_test 2>&1 | tail -5`
Expected: 1 passed.

- [ ] **Step 5: Commit and open PR-1**

```bash
git add sntl/src/lib.rs sntl/tests/types_reexport_test.rs
git commit -m "feat(sntl): re-export driver hstore/ltree/cube modules under sntl::types"
git push -u origin feat/cluster-a-arrays
gh pr create --title "feat(sntl): re-export driver extension types" \
  --body "Surfaces hstore (and ltree/cube where present) under sntl::types::* so consumers can reach them without depending on sentinel-driver directly."
```

> Merge PR-1 before continuing. PR-2 builds on a clean `sntl::types` namespace.

---

## Phase 2 — sentinel array work (PR-2)

### Task 6: Add `ElementTypeRef` + `ColumnInfo.element_type`

**Files:**
- Modify: `sntl-schema/src/cache.rs`

- [ ] **Step 1: Define `ElementTypeRef` and extend `ColumnInfo`**

In `sntl-schema/src/cache.rs`, after the `ColumnOrigin` struct and before `QueryKind`:

```rust
/// Per-element type info for array columns. `None` (default) means the
/// column is not an array, or the cache was generated before this field
/// existed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElementTypeRef {
    pub pg_type: String,
    pub oid: u32,
}
```

Modify `ColumnInfo` (lines ~34-43) to add the new field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub pg_type: String,
    pub oid: u32,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub origin: Option<ColumnOrigin>,
    /// `Some` for array columns. Backward-compatible default = `None`.
    #[serde(default)]
    pub element_type: Option<ElementTypeRef>,
}
```

- [ ] **Step 2: Verify compile**

Run: `cargo check -p sntl-schema 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add sntl-schema/src/cache.rs
git commit -m "feat(sntl-schema): add ElementTypeRef + ColumnInfo.element_type (additive)"
```

---

### Task 7: Backward-compat tests for the new field

**Files:**
- Create: `sntl-schema/tests/cache_element_type_test.rs`

- [ ] **Step 1: Write the failing tests**

Create `sntl-schema/tests/cache_element_type_test.rs`:

```rust
use sntl_schema::cache::{Cache, CacheEntry, ColumnInfo, ElementTypeRef, ParamInfo, QueryKind};
use tempfile::tempdir;

fn entry_with_array_column() -> CacheEntry {
    CacheEntry {
        version: 1,
        sql_hash: "arr1".into(),
        sql_normalized: "SELECT tags FROM users".into(),
        source_locations: vec![],
        params: vec![],
        columns: vec![ColumnInfo {
            name: "tags".into(),
            pg_type: "_text".into(),
            oid: 1009,
            nullable: false,
            origin: None,
            element_type: Some(ElementTypeRef {
                pg_type: "text".into(),
                oid: 25,
            }),
        }],
        query_kind: QueryKind::Select,
        has_returning: false,
    }
}

#[test]
fn element_type_roundtrip() {
    let dir = tempdir().unwrap();
    let cache = Cache::new(dir.path());
    cache.init().unwrap();
    let entry = entry_with_array_column();
    cache.write_entry(&entry).unwrap();
    let loaded = cache.read_entry("arr1").unwrap();
    assert_eq!(
        loaded.columns[0].element_type,
        Some(ElementTypeRef {
            pg_type: "text".into(),
            oid: 25,
        })
    );
}

#[test]
fn old_cache_without_element_type_deserialises() {
    let dir = tempdir().unwrap();
    let cache = Cache::new(dir.path());
    cache.init().unwrap();

    // Hand-write a v1 cache file from before this field existed.
    let path = dir.path().join("queries").join("legacy.json");
    std::fs::write(
        &path,
        r#"{
            "version": 1,
            "sql_hash": "legacy",
            "sql_normalized": "SELECT id FROM users",
            "params": [],
            "columns": [{
                "name": "id",
                "pg_type": "int4",
                "oid": 23,
                "nullable": false
            }],
            "query_kind": "Select",
            "has_returning": false
        }"#,
    )
    .unwrap();

    let loaded = cache.read_entry("legacy").unwrap();
    assert!(loaded.columns[0].element_type.is_none(), "missing field must default to None");
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p sntl-schema --test cache_element_type_test 2>&1 | tail -8`
Expected: 2 passed.

- [ ] **Step 3: Commit**

```bash
git add sntl-schema/tests/cache_element_type_test.rs
git commit -m "test(sntl-schema): roundtrip + backward-compat for ColumnInfo.element_type"
```

---

### Task 8: Extend `ResolveInput` and `resolve_offline` with element overrides

**Files:**
- Modify: `sntl-schema/src/resolve.rs`

- [ ] **Step 1: Add the new field**

In `sntl-schema/src/resolve.rs`, modify `ResolveInput`:

```rust
pub struct ResolveInput<'a> {
    pub sql: &'a str,
    pub cache_entry: &'a CacheEntry,
    pub schema: &'a Schema,
    pub overrides_nullable: &'a [String],
    pub overrides_non_null: &'a [String],
    /// Names of array columns whose elements the caller asserts are non-null.
    pub overrides_non_null_elements: &'a [String],
    pub strict: bool,
}
```

`ResolvedQuery` itself does not need a new field — element nullability is communicated implicitly through `ColumnInfo.element_type` and the override set.

- [ ] **Step 2: Add validation logic to `resolve_offline`**

Inside `resolve_offline`, after the existing override-validation loop, add:

```rust
// Validate element overrides reference real array columns
for name in input.overrides_non_null_elements.iter() {
    let col = columns
        .iter()
        .find(|c| &c.name == name)
        .ok_or_else(|| Error::Config(format!(
            "override `non_null_elements` references unknown column `{name}`"
        )))?;
    if col.element_type.is_none() {
        return Err(Error::Config(format!(
            "override `non_null_elements` references `{name}` which is not an array column"
        )));
    }
}
```

Note: the overrides themselves do not mutate `ColumnInfo` — codegen consults the override list directly. This keeps `ResolvedQuery` shape stable.

Expose `overrides_non_null_elements` on `ResolvedQuery` so codegen can read it without re-receiving the input:

```rust
pub struct ResolvedQuery {
    pub params: Vec<ParamInfo>,
    pub columns: Vec<ColumnInfo>,
    pub query_kind: QueryKind,
    pub has_returning: bool,
    /// Forwarded for codegen so it can decide Vec<T> vs Vec<Option<T>>.
    pub non_null_elements: Vec<String>,
}
```

And populate it at the end of `resolve_offline`:

```rust
Ok(ResolvedQuery {
    params: input.cache_entry.params.clone(),
    columns,
    query_kind: input.cache_entry.query_kind,
    has_returning: input.cache_entry.has_returning,
    non_null_elements: input.overrides_non_null_elements.to_vec(),
})
```

- [ ] **Step 3: Update existing call sites that construct `ResolveInput`**

Open `sntl-macros/src/query/anonymous.rs`, `typed.rs`, `file.rs`, `pipeline.rs`. Each builds a `ResolveInput { … }` literal — add `overrides_non_null_elements: &[]` to every literal so the change compiles. (The macro-side parser changes that wire up the actual override come in Task 12.)

```rust
let resolved = match resolve_offline(ResolveInput {
    sql: &sql,
    cache_entry: &entry,
    schema: &schema,
    overrides_nullable: &nullable,
    overrides_non_null: &non_null,
    overrides_non_null_elements: &[],   // NEW — wired up in Task 12
    strict: true,
}) { … }
```

- [ ] **Step 4: Build**

Run: `cargo check --workspace 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add sntl-schema/src/resolve.rs sntl-macros/src/query/
git commit -m "feat(sntl-schema): ResolveInput.overrides_non_null_elements + ResolvedQuery field"
```

---

### Task 9: Tests for element override validation

**Files:**
- Create: `sntl-schema/tests/resolve_element_test.rs`

- [ ] **Step 1: Write the failing tests**

Create `sntl-schema/tests/resolve_element_test.rs`:

```rust
use sntl_schema::cache::{CacheEntry, ColumnInfo, ElementTypeRef, ParamInfo, QueryKind};
use sntl_schema::resolve::{ResolveInput, resolve_offline};
use sntl_schema::schema::{Column, PgTypeRef, Schema, Table};

fn schema_with_users() -> Schema {
    Schema {
        version: 1,
        postgres_version: "16".into(),
        generated_at: None,
        source: None,
        tables: vec![Table {
            name: "users".into(),
            schema: "public".into(),
            columns: vec![
                Column { name: "id".into(), pg_type: PgTypeRef::simple("int4"), oid: 23, nullable: false, primary_key: true, unique: false, default: None },
                Column { name: "tags".into(), pg_type: PgTypeRef::simple("_text"), oid: 1009, nullable: false, primary_key: false, unique: false, default: None },
            ],
            foreign_keys: vec![],
        }],
        enums: vec![],
        composites: vec![],
    }
}

fn entry_with_tags() -> CacheEntry {
    CacheEntry {
        version: 1,
        sql_hash: "tags1".into(),
        sql_normalized: "SELECT id, tags FROM users".into(),
        source_locations: vec![],
        params: vec![],
        columns: vec![
            ColumnInfo {
                name: "id".into(),
                pg_type: "int4".into(),
                oid: 23,
                nullable: false,
                origin: None,
                element_type: None,
            },
            ColumnInfo {
                name: "tags".into(),
                pg_type: "_text".into(),
                oid: 1009,
                nullable: false,
                origin: None,
                element_type: Some(ElementTypeRef { pg_type: "text".into(), oid: 25 }),
            },
        ],
        query_kind: QueryKind::Select,
        has_returning: false,
    }
}

#[test]
fn override_passes_through_when_column_is_array() {
    let schema = schema_with_users();
    let entry = entry_with_tags();
    let r = resolve_offline(ResolveInput {
        sql: "SELECT id, tags FROM users",
        cache_entry: &entry,
        schema: &schema,
        overrides_nullable: &[],
        overrides_non_null: &[],
        overrides_non_null_elements: &["tags".to_string()],
        strict: true,
    })
    .unwrap();
    assert_eq!(r.non_null_elements, vec!["tags".to_string()]);
}

#[test]
fn rejects_override_on_non_array_column() {
    let schema = schema_with_users();
    let entry = entry_with_tags();
    let err = resolve_offline(ResolveInput {
        sql: "SELECT id, tags FROM users",
        cache_entry: &entry,
        schema: &schema,
        overrides_nullable: &[],
        overrides_non_null: &[],
        overrides_non_null_elements: &["id".to_string()],
        strict: true,
    })
    .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("not an array column"), "{msg}");
}

#[test]
fn rejects_override_on_unknown_column() {
    let schema = schema_with_users();
    let entry = entry_with_tags();
    let err = resolve_offline(ResolveInput {
        sql: "SELECT id, tags FROM users",
        cache_entry: &entry,
        schema: &schema,
        overrides_nullable: &[],
        overrides_non_null: &[],
        overrides_non_null_elements: &["bogus".to_string()],
        strict: true,
    })
    .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("unknown column"), "{msg}");
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p sntl-schema --test resolve_element_test 2>&1 | tail -8`
Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add sntl-schema/tests/resolve_element_test.rs
git commit -m "test(sntl-schema): override validation for non_null_elements"
```

---

### Task 10: Batch element-type lookup in `introspect`

**Files:**
- Modify: `sntl-schema/src/introspect.rs`

- [ ] **Step 1: Add a batched lookup helper**

In `sntl-schema/src/introspect.rs`, add this helper near the top of the file (after the imports):

```rust
use crate::cache::ElementTypeRef;
use std::collections::HashMap;

/// Look up element types for a set of array OIDs in one round-trip.
/// Returns an empty map if `array_oids` is empty.
async fn fetch_element_types(
    client: &mut sentinel_driver::Connection,
    array_oids: &[u32],
) -> Result<HashMap<u32, ElementTypeRef>> {
    if array_oids.is_empty() {
        return Ok(HashMap::new());
    }
    let oids_i32: Vec<i32> = array_oids.iter().map(|o| *o as i32).collect();
    let rows = client
        .query(
            "SELECT t.oid::int4 AS array_oid, e.oid::int4 AS elem_oid, e.typname \
             FROM pg_catalog.pg_type t \
             JOIN pg_catalog.pg_type e ON e.oid = t.typelem \
             WHERE t.oid = ANY($1) AND t.typelem <> 0",
            &[&oids_i32.as_slice()],
        )
        .await
        .map_err(|e| Error::Introspect(format!("element-type lookup: {e}")))?;

    let mut out = HashMap::new();
    for row in rows {
        let array_oid: i32 = row
            .try_get(0)
            .map_err(|e| Error::Introspect(format!("decode array_oid: {e}")))?;
        let elem_oid: i32 = row
            .try_get(1)
            .map_err(|e| Error::Introspect(format!("decode elem_oid: {e}")))?;
        let pg_type: String = row
            .try_get(2)
            .map_err(|e| Error::Introspect(format!("decode typname: {e}")))?;
        out.insert(
            array_oid as u32,
            ElementTypeRef {
                pg_type,
                oid: elem_oid as u32,
            },
        );
    }
    Ok(out)
}
```

- [ ] **Step 2: Wire it into `prepare_query`**

In the same file, modify `prepare_query` so that after building the `columns: Vec<ColumnInfo>` from `stmt.columns()`, it calls `fetch_element_types` and patches `element_type` on each matching column:

```rust
let mut columns: Vec<ColumnInfo> = stmt
    .columns()
    .map(<[_]>::to_vec)
    .unwrap_or_default()
    .into_iter()
    .map(|c| ColumnInfo {
        name: c.name,
        pg_type: String::new(),
        oid: c.type_oid,
        nullable: true,
        origin: None,
        element_type: None, // populated below
    })
    .collect();

// Patch in element types for any columns whose oid is an array oid.
let array_oids: Vec<u32> = columns.iter().map(|c| c.oid).collect();
let elements = fetch_element_types(&mut client, &array_oids).await?;
for col in columns.iter_mut() {
    if let Some(et) = elements.get(&col.oid) {
        col.element_type = Some(et.clone());
    }
}
```

- [ ] **Step 3: Verify compile (live-DB tests are exercised in Task 11)**

Run: `cargo check -p sntl-schema --all-features 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add sntl-schema/src/introspect.rs
git commit -m "feat(sntl-schema): introspect populates ColumnInfo.element_type via batch pg_type query"
```

---

### Task 11: Sanity-check `sntl prepare` against a live PG

> **Prerequisite:** Postgres available at `DATABASE_URL`. The PR-#12 setup `tests/integration/setup.sql` does not yet have a `tags` column; that change ships in Task 18 alongside the integration test. Skip this task in CI; run it locally before merging Phase 2.

- [ ] **Step 1: Add a temporary array column**

```bash
podman exec -i sntl-test-pg psql -U sentinel -d sentinel_test -c \
  "ALTER TABLE users ADD COLUMN IF NOT EXISTS tags text[]"
```

- [ ] **Step 2: Run `sntl prepare` against a tiny test workspace**

Create a scratch `examples/array_probe.rs`:

```rust
fn main() {
    // Just exists so sntl prepare's scanner sees the SQL string.
    let _ = sntl::query!("SELECT tags FROM users");
}
```

Run:

```bash
DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test \
  cargo run -p sntl-cli -- prepare
cat .sentinel/queries/$(cargo run -p sntl-schema --example compute_hash -q -- 'SELECT tags FROM users').json | head -30
```

Expected: the cache file's `columns[0].element_type` is populated with `{"pg_type":"text","oid":25}`.

- [ ] **Step 3: Clean up the scratch file and commit nothing**

This is a manual smoke test — no commit. Delete `examples/array_probe.rs`.

---

### Task 12: Macro `args.rs` — `non_null_elements` keyword

**Files:**
- Modify: `sntl-macros/src/query/args.rs`

- [ ] **Step 1: Extend `QueryArgs`**

Modify `QueryArgs`:

```rust
pub struct QueryArgs {
    pub sql: LitStr,
    pub params: Vec<Expr>,
    pub overrides_nullable: Vec<Ident>,
    pub overrides_non_null: Vec<Ident>,
    pub overrides_non_null_elements: Vec<Ident>,
}
```

- [ ] **Step 2: Parse the new keyword**

In the `Parse for QueryArgs` `match key.to_string().as_str()` block, add a third arm:

```rust
"non_null_elements" => {
    let _key: Ident = input.parse()?;
    input.parse::<Token![=]>()?;
    overrides_non_null_elements = parse_ident_list(input)?.into_iter().collect();
    continue;
}
```

Update the constructor at the end of `parse`:

```rust
Ok(QueryArgs {
    sql,
    params,
    overrides_nullable,
    overrides_non_null,
    overrides_non_null_elements,
})
```

- [ ] **Step 3: Wire through `anonymous.rs`, `typed.rs`, `file.rs`**

In each `expand*` function, replace the placeholder `&[]` from Task 8 with the actual override list:

```rust
let non_null_elements = idents_to_strings(&args.overrides_non_null_elements);

let resolved = match resolve_offline(ResolveInput {
    sql: &sql,
    cache_entry: &entry,
    schema: &schema,
    overrides_nullable: &nullable,
    overrides_non_null: &non_null,
    overrides_non_null_elements: &non_null_elements,
    strict: true,
}) { … }
```

For `query_as_args` (typed.rs), `args.query.overrides_non_null_elements` etc.

`file.rs` forwards to `query!` / `query_as!` — extend its `quote!` to pass `non_null_elements = [#(#non_null_elements),*]`.

- [ ] **Step 4: Build**

Run: `cargo check -p sntl-macros 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add sntl-macros/src/query/
git commit -m "feat(sntl-macros): non_null_elements keyword wired through every query!() variant"
```

---

### Task 13: codegen branches on `element_type`

**Files:**
- Modify: `sntl-macros/src/query/codegen.rs`

- [ ] **Step 1: Replace `rust_type_for_column` with the array-aware version**

Replace the existing `rust_type_for_column` with:

```rust
pub fn rust_type_for_column(c: &ColumnInfo, non_null_elements: &[String]) -> TokenStream {
    if let Some(elem) = &c.element_type {
        let element_non_null = non_null_elements.iter().any(|n| n == &c.name);
        let elem_ty = rust_type_for_pg_oid(elem.oid, &elem.pg_type);
        let inner = if element_non_null {
            quote! { #elem_ty }
        } else {
            quote! { ::std::option::Option<#elem_ty> }
        };
        let array_ty = quote! { ::std::vec::Vec<#inner> };
        if c.nullable {
            quote! { ::std::option::Option<#array_ty> }
        } else {
            array_ty
        }
    } else {
        let base = rust_type_for_pg_oid(c.oid, &c.pg_type);
        if c.nullable {
            quote! { ::std::option::Option<#base> }
        } else {
            base
        }
    }
}
```

- [ ] **Step 2: Update every `rust_type_for_column` call site**

In `anonymous.rs` and `typed.rs::expand_scalar`, the previous one-arg call becomes:

```rust
let ty = rust_type_for_column(c, &resolved.non_null_elements);
```

Pass `&resolved.non_null_elements` everywhere `rust_type_for_column(c)` was called.

- [ ] **Step 3: Build**

Run: `cargo check -p sntl-macros 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add sntl-macros/src/query/codegen.rs sntl-macros/src/query/anonymous.rs sntl-macros/src/query/typed.rs
git commit -m "feat(sntl-macros): codegen emits Vec<Option<T>>/Vec<T> based on element_type + override"
```

---

### Task 14: Trybuild fixtures for array work

**Files:**
- Create: `sntl-macros/tests/expand/query/array_basic.rs`
- Create: `sntl-macros/tests/expand/query/array_non_null.rs`
- Create: `sntl-macros/tests/expand/query/non_null_elements_bad.rs`
- Modify: `sntl-macros/tests/query_expand.rs`

- [ ] **Step 1: Write the pass fixtures**

`sntl-macros/tests/expand/query/array_basic.rs`:

```rust
// Pass-case: array column emits Vec<Option<T>> by default.

#[cfg(feature = "trybuild_fixtures")]
fn main() {
    async fn demo(conn: &mut sntl::driver::Connection) -> sntl::Result<()> {
        let row = sntl::query!("SELECT tags FROM users").fetch_one(conn).await?;
        let _: Vec<Option<String>> = row.tags;
        Ok(())
    }
    let _ = demo;
}

#[cfg(not(feature = "trybuild_fixtures"))]
fn main() {}
```

`sntl-macros/tests/expand/query/array_non_null.rs`:

```rust
// Pass-case: non_null_elements override emits Vec<T>.

#[cfg(feature = "trybuild_fixtures")]
fn main() {
    async fn demo(conn: &mut sntl::driver::Connection) -> sntl::Result<()> {
        let row = sntl::query!(
            "SELECT tags FROM users",
            non_null_elements = [tags]
        )
        .fetch_one(conn)
        .await?;
        let _: Vec<String> = row.tags;
        Ok(())
    }
    let _ = demo;
}

#[cfg(not(feature = "trybuild_fixtures"))]
fn main() {}
```

`sntl-macros/tests/expand/query/non_null_elements_bad.rs`:

```rust
fn main() {
    let _ = sntl::query!(
        "SELECT id FROM users WHERE id = $1",
        1i32,
        non_null_elements = [id]
    );
}
```

- [ ] **Step 2: Register the new cases**

In `sntl-macros/tests/query_expand.rs`:

```rust
#[test]
fn query_expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/query/basic.rs");
    t.pass("tests/expand/query/array_basic.rs");
    t.pass("tests/expand/query/array_non_null.rs");
    t.compile_fail("tests/expand/query/cache_miss.rs");
    t.compile_fail("tests/expand/query/non_null_elements_bad.rs");
}
```

- [ ] **Step 3: Seed cache fixtures for the two new pass cases**

Compute hashes:

```bash
HASH_ARR=$(cargo run -p sntl-schema --example compute_hash -q -- "SELECT tags FROM users")
echo "$HASH_ARR"
```

Create `.sentinel/queries/$HASH_ARR.json`:

```json
{
  "version": 1,
  "sql_hash": "<HASH_ARR>",
  "sql_normalized": "SELECT tags FROM users",
  "params": [],
  "columns": [
    {
      "name": "tags",
      "pg_type": "_text",
      "oid": 1009,
      "nullable": false,
      "element_type": { "pg_type": "text", "oid": 25 }
    }
  ],
  "query_kind": "Select",
  "has_returning": false
}
```

The same hash + JSON serves both `array_basic.rs` and `array_non_null.rs` — they share the SQL string. (Cache lookup is keyed on the normalised SQL hash, so the two fixture files reuse one entry.)

For `non_null_elements_bad.rs` the SQL string `"SELECT id FROM users WHERE id = $1"` already has a cache entry from PR #12 — no new fixture needed.

Update `.sentinel/schema.toml`'s users table to include the `tags` column (additive — does not affect the existing PR-#12 fixture):

```toml
  [[tables.columns]]
  name = "tags"
  pg_type = "_text"
  oid = 1009
  nullable = false
```

- [ ] **Step 4: Run**

Run: `cargo test -p sntl-macros --features trybuild_fixtures --test query_expand 2>&1 | tail -10`
Expected: pass cases compile; the `non_null_elements_bad.rs` case generates a `wip/non_null_elements_bad.stderr`. Move it:

```bash
mv sntl-macros/wip/non_null_elements_bad.stderr sntl-macros/tests/expand/query/non_null_elements_bad.stderr
```

Re-run the test — it should pass with the snapshot in place.

- [ ] **Step 5: Commit**

```bash
git add sntl-macros/tests/ .sentinel/
git commit -m "test(sntl-macros): trybuild fixtures for array element nullability + override"
```

---

### Task 15: Live-PG integration test for array round-trip

**Files:**
- Modify: `tests/integration/setup.sql`
- Create: `sntl/tests/macro_array_test.rs`

- [ ] **Step 1: Add the column to the integration schema**

Append to `tests/integration/setup.sql` after the `users` table is created:

```sql
ALTER TABLE users ADD COLUMN tags text[];
```

- [ ] **Step 2: Write the test**

Create `sntl/tests/macro_array_test.rs`:

```rust
mod pg_helpers;

use sntl::driver::{Config, Connection};

#[tokio::test]
async fn array_with_null_elements_roundtrips() {
    let url = match std::env::var("DATABASE_URL").ok() {
        Some(u) => u,
        None => return,
    };
    let mut conn = Connection::connect(Config::parse(&url).expect("parse")).await.expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    conn.execute(
        "INSERT INTO users (name, email, tags) VALUES ($1, $2, ARRAY['a', NULL, 'b']::text[])",
        &[&"u1", &"u1@example.com"],
    )
    .await
    .unwrap();

    let row = sntl::query!("SELECT tags FROM users").fetch_one(&mut conn).await.unwrap();
    assert_eq!(row.tags, vec![Some("a".to_string()), None, Some("b".to_string())]);
}

#[tokio::test]
async fn array_non_null_override_emits_vec_t() {
    let url = match std::env::var("DATABASE_URL").ok() {
        Some(u) => u,
        None => return,
    };
    let mut conn = Connection::connect(Config::parse(&url).expect("parse")).await.expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    conn.execute(
        "INSERT INTO users (name, email, tags) VALUES ($1, $2, ARRAY['a', 'b']::text[])",
        &[&"u2", &"u2@example.com"],
    )
    .await
    .unwrap();

    let row = sntl::query!(
        "SELECT tags FROM users",
        non_null_elements = [tags]
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(row.tags, vec!["a".to_string(), "b".to_string()]);
}

#[tokio::test]
async fn array_non_null_override_errors_on_actual_null() {
    let url = match std::env::var("DATABASE_URL").ok() {
        Some(u) => u,
        None => return,
    };
    let mut conn = Connection::connect(Config::parse(&url).expect("parse")).await.expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    conn.execute(
        "INSERT INTO users (name, email, tags) VALUES ($1, $2, ARRAY['a', NULL]::text[])",
        &[&"u3", &"u3@example.com"],
    )
    .await
    .unwrap();

    let err = sntl::query!(
        "SELECT tags FROM users",
        non_null_elements = [tags]
    )
    .fetch_one(&mut conn)
    .await
    .expect_err("decoding NULL into Vec<T> should error");

    let msg = format!("{err}");
    assert!(msg.contains("NULL elements not supported"), "{msg}");
}
```

- [ ] **Step 3: Run locally with PG**

```bash
podman exec -i sntl-test-pg psql -U sentinel -d sentinel_test < tests/integration/setup.sql
DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test \
  cargo test -p sntl --test macro_array_test 2>&1 | tail -10
```

Expected: 3 passed.

- [ ] **Step 4: Commit + open PR-2**

```bash
git add tests/integration/setup.sql sntl/tests/macro_array_test.rs
git commit -m "test(sntl): live-PG round-trip for Vec<Option<T>> + non_null_elements override"
git push
gh pr create --title "feat(sntl): auto-emit Vec<Option<T>> for array columns + non_null_elements override" \
  --body "Implements Phase 2 of the cluster-A plan (docs/plans/2026-04-29-cluster-a-array-tuple-design.md). Depends on sentinel-driver 1.0.1 (Phase 0)."
```

> Merge PR-2 before continuing. PR-3 is independent of PR-2 but easier to review on a clean main.

---

## Phase 3 — Tuple FromRow (PR-3)

### Task 16: Blanket `FromRow` for tuple arities 1–16

**Files:**
- Modify: `sntl/src/core/query/macro_support.rs`

- [ ] **Step 1: Add the macro and 16 invocations**

Append to `sntl/src/core/query/macro_support.rs`:

```rust
macro_rules! impl_from_row_tuple {
    ($($ty:ident at $idx:tt),+ $(,)?) => {
        impl<$($ty),+> FromRow for ($($ty,)+)
        where
            $($ty: ::driver::FromSql),+
        {
            fn from_row(row: &::driver::Row) -> $crate::core::error::Result<Self> {
                Ok(($(
                    row.try_get::<$ty>($idx)
                        .map_err(|e| $crate::Error::Driver(e))?,
                )+))
            }
        }
    };
}

impl_from_row_tuple!(T0 at 0);
impl_from_row_tuple!(T0 at 0, T1 at 1);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7, T8 at 8);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7, T8 at 8, T9 at 9);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7, T8 at 8, T9 at 9, T10 at 10);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7, T8 at 8, T9 at 9, T10 at 10, T11 at 11);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7, T8 at 8, T9 at 9, T10 at 10, T11 at 11, T12 at 12);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7, T8 at 8, T9 at 9, T10 at 10, T11 at 11, T12 at 12, T13 at 13);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7, T8 at 8, T9 at 9, T10 at 10, T11 at 11, T12 at 12, T13 at 13, T14 at 14);
impl_from_row_tuple!(T0 at 0, T1 at 1, T2 at 2, T3 at 3, T4 at 4, T5 at 5, T6 at 6, T7 at 7, T8 at 8, T9 at 9, T10 at 10, T11 at 11, T12 at 12, T13 at 13, T14 at 14, T15 at 15);
```

- [ ] **Step 2: Build**

Run: `cargo check -p sntl 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add sntl/src/core/query/macro_support.rs
git commit -m "feat(sntl): blanket FromRow impls for tuples (arities 1-16)"
```

---

### Task 17: `query_as!` tuple arity validation

**Files:**
- Modify: `sntl-macros/src/query/typed.rs`

- [ ] **Step 1: Detect the tuple form and check arity**

In `expand_as` (top of the function, after `let target = &args.target;`):

```rust
// If the target syn::Path is actually a tuple type, syn will surface it
// as a path that doesn't parse — query_as!((i32, String), …) reaches us
// as a Type rather than Path. Require the parser in args.rs to keep
// `target: syn::Type` (change there too). Then:
if let syn::Type::Tuple(tup) = &args.target {
    let actual = tup.elems.len();
    let expected = resolved.columns.len();
    if actual != expected {
        let cols = resolved
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        proc_macro_error2::abort!(
            tup,
            "query_as! tuple arity mismatch: tuple expects {} columns, SELECT returns {} ({})",
            actual,
            expected,
            cols
        );
    }
}
```

This requires changing `QueryAsArgs` (and `parse_query_as_args`) in `args.rs` from `pub target: Path` to `pub target: syn::Type`. Update `expand_as` to use `&args.target` as a `Type` everywhere it currently expects a `Path` — the existing `quote! { #target }` continues to work because `Type` implements `ToTokens`.

- [ ] **Step 2: Update the assert helper**

The previous `_assert_from_row::<#target>()` works for any `T: FromRow`, including tuples. No change needed there.

- [ ] **Step 3: Build**

Run: `cargo check --workspace 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add sntl-macros/src/query/args.rs sntl-macros/src/query/typed.rs
git commit -m "feat(sntl-macros): query_as! accepts tuple targets + arity check"
```

---

### Task 18: Trybuild fixtures for tuple FromRow

**Files:**
- Create: `sntl-macros/tests/expand/query/tuple_basic.rs`
- Create: `sntl-macros/tests/expand/query/tuple_arity_bad.rs`
- Modify: `sntl-macros/tests/query_expand.rs`

- [ ] **Step 1: Write the fixtures**

`sntl-macros/tests/expand/query/tuple_basic.rs`:

```rust
#[cfg(feature = "trybuild_fixtures")]
fn main() {
    async fn demo(conn: &mut sntl::driver::Connection) -> sntl::Result<()> {
        let id: i32 = 1;
        let _: (i32,) = sntl::query_as!(
            (i32,),
            "SELECT id FROM users WHERE id = $1",
            id
        )
        .fetch_one(conn)
        .await?;
        Ok(())
    }
    let _ = demo;
}

#[cfg(not(feature = "trybuild_fixtures"))]
fn main() {}
```

`sntl-macros/tests/expand/query/tuple_arity_bad.rs`:

```rust
fn main() {
    let id: i32 = 1;
    let _ = sntl::query_as!(
        (i32, String),
        "SELECT id FROM users WHERE id = $1",
        id
    );
}
```

- [ ] **Step 2: Register**

`sntl-macros/tests/query_expand.rs`:

```rust
#[test]
fn query_expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/query/basic.rs");
    t.pass("tests/expand/query/array_basic.rs");
    t.pass("tests/expand/query/array_non_null.rs");
    t.pass("tests/expand/query/tuple_basic.rs");
    t.compile_fail("tests/expand/query/cache_miss.rs");
    t.compile_fail("tests/expand/query/non_null_elements_bad.rs");
    t.compile_fail("tests/expand/query/tuple_arity_bad.rs");
}
```

The cache entry for `"SELECT id FROM users WHERE id = $1"` already exists (PR #12) — both the pass case and the arity-mismatch case use it. The arity case asserts that the macro errors *before* getting to runtime, since the cached column count (1) disagrees with the requested tuple arity (2).

- [ ] **Step 3: Run + capture stderr snapshot**

```bash
cargo test -p sntl-macros --features trybuild_fixtures --test query_expand 2>&1 | tail -10
mv sntl-macros/wip/tuple_arity_bad.stderr sntl-macros/tests/expand/query/tuple_arity_bad.stderr 2>/dev/null || true
cargo test -p sntl-macros --features trybuild_fixtures --test query_expand 2>&1 | tail -5
```

Expected: all pass cases compile; both compile_fail cases match snapshots.

- [ ] **Step 4: Commit**

```bash
git add sntl-macros/tests/
git commit -m "test(sntl-macros): trybuild fixtures for tuple FromRow + arity mismatch"
```

---

### Task 19: Live-PG integration test for tuple

**Files:**
- Create: `sntl/tests/macro_tuple_from_row_test.rs`

- [ ] **Step 1: Write the test**

```rust
mod pg_helpers;

use sntl::driver::{Config, Connection};

#[tokio::test]
async fn tuple_query_as_returns_tuple_directly() {
    let url = match std::env::var("DATABASE_URL").ok() {
        Some(u) => u,
        None => return,
    };
    let mut conn = Connection::connect(Config::parse(&url).expect("parse")).await.expect("connect");
    pg_helpers::clean_tables(&mut conn).await;

    let inserted_id: i32 = conn
        .query_one(
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id",
            &[&"tuple-test", &"tuple@example.com"],
        )
        .await
        .unwrap()
        .try_get(0)
        .unwrap();

    let (id,): (i32,) = sntl::query_as!(
        (i32,),
        "SELECT id FROM users WHERE id = $1",
        inserted_id
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(id, inserted_id);
}
```

- [ ] **Step 2: Run**

```bash
DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test \
  cargo test -p sntl --test macro_tuple_from_row_test 2>&1 | tail -5
```

Expected: 1 passed.

- [ ] **Step 3: Commit + open PR-3**

```bash
git add sntl/tests/macro_tuple_from_row_test.rs
git commit -m "test(sntl): live-PG round-trip for tuple FromRow target"
git push
gh pr create --title "feat(sntl): blanket FromRow impls for tuples (arities 1-16) + arity check" \
  --body "Implements Phase 3 of the cluster-A plan. Independent of PR-2; can land in either order."
```

---

## Self-Review

**Spec coverage check** (against `docs/plans/2026-04-29-cluster-a-array-tuple-design.md`):

| Spec section | Implementing task(s) |
|---|---|
| §1 driver `Decode for Vec<Option<T>>` | Tasks 1, 2, 3 |
| §1 `PgHasArrayType` parity | Task 2 (the macro generates both `oid()` returns the same array oid for `Vec<Option<T>>` as for `Vec<T>`, so the existing `PgHasArrayType` derivation through the `oid()` constant works) |
| §1 macro auto-emits Vec<Option<T>> | Tasks 12, 13 |
| §1 `non_null_elements` keyword | Tasks 8, 9, 12, 13 |
| §1 plain tuple FromRow | Tasks 16, 17 |
| §1 hstore/ltree/cube re-export | Task 5 |
| §1 backward-compat additive cache field | Tasks 6, 7 |
| §2 sntl prepare populates element_type | Tasks 10, 11 |
| §2 introspect via batch pg_type query | Task 10 |
| §3 type emission matrix | Task 13 (`rust_type_for_column` covers all 8 rows) |
| §3 override validation (non-array, unknown column) | Tasks 8, 9; trybuild Task 14 |
| §3 tuple FromRow placement (positional) | Task 16 |
| §3 query_as! tuple arity check | Task 17; trybuild Task 18 |
| §4 driver synthesised + live-PG tests | Task 3 |
| §4 trybuild pass + compile_fail fixtures | Tasks 14, 18 |
| §4 live-PG integration tests | Tasks 15, 19 |
| §5 cadence (4 PRs in order) | Phase headers + PR-open steps in Tasks 5, 15, 19 |
| §6 open items resolved | Task 10 batches via `ANY($1)`; element type comes from `pg_catalog.pg_type` join (live-DB lookup, not hardcoded); `Connection::prepare` provides type_oid, no extra metadata needed |

**Placeholder scan:** none — every task has concrete code, commands, and expected output.

**Type consistency check:**

- `ColumnInfo.element_type: Option<ElementTypeRef>` — defined in Task 6, consumed in Tasks 7, 9, 10, 13, 14, 15.
- `ResolveInput.overrides_non_null_elements: &[String]` — added in Task 8, populated in Task 12, every existing call site updated in Task 8.
- `ResolvedQuery.non_null_elements: Vec<String>` — added in Task 8, consumed in Task 13.
- `rust_type_for_column(c, non_null_elements)` — new signature in Task 13, all call sites updated.
- `QueryArgs.overrides_non_null_elements` — added in Task 12; every `expand*` already wired to `&[]` placeholder in Task 8 → upgraded in Task 12.
- `QueryAsArgs.target` changes from `Path` to `Type` in Task 17 — all uses of `&args.target` work because `Type` implements `ToTokens`.
- Driver `decode_array_inner` (Task 1) is the foundation; both `decode_array` and `decode_array_nullable` build on it; `impl_array_from_sql!` (Task 2) generates both `Vec<T>` and `Vec<Option<T>>` via those two helpers.

**Open items** — none. The spec's three open items are resolved in this plan: batch via `ANY($1)`, element types via `pg_type` join, and `Connection::prepare` is sufficient (no extra introspection needed beyond OID).

---

## Execution handoff

Plan complete and saved to `docs/plans/2026-04-29-cluster-a-array-tuple-impl.md`. Two execution options:

**1. Subagent-Driven (recommended)** — fresh subagent per task, two-stage review, fast iteration.

**2. Inline Execution** — run tasks in this session with checkpoints.

Which approach?
