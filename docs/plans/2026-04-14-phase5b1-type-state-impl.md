# Phase 5B-1: Type-State Pattern (Flat Includes) — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Compile-time N+1 prevention — accessing a relation that wasn't `.Include()`d is a compile error with a helpful diagnostic message.

**Architecture:** Library core types (`WithRelations`, `IncludeQuery`, `RelationLoaded` trait) in `sntl/src/core/`, macro generates per-model glue (markers, transitions, accessors). `ModelQuery` gains `<M>` generic param (backward compatible via default). Flat includes only (1 level deep).

**Tech Stack:** Rust proc macros (syn/quote), `#[diagnostic::on_unimplemented]` (stable since 1.78, project requires 1.85)

---

### Task 1: Add `RelationStore` and `WithRelations` to library core

**Files:**
- Modify: `sntl/src/core/relation.rs`
- Test: `sntl/tests/with_relations_test.rs`

**Step 1: Write failing tests**

Create `sntl/tests/with_relations_test.rs`:

```rust
use sntl::core::relation::{RelationStore, WithRelations, Loaded, Unloaded};

#[test]
fn with_relations_deref_to_model() {
    let model = SimpleModel { id: 1, name: "test".into() };
    let wr: WithRelations<SimpleModel, (Unloaded,)> = WithRelations::bare(model);
    // Deref — access model fields directly
    assert_eq!(wr.id, 1);
    assert_eq!(wr.name, "test");
}

#[test]
fn relation_store_insert_and_get() {
    let mut store = RelationStore::new();
    assert!(store.is_empty());
    store.insert_decoded("posts", vec![42i32, 43, 44]);
    assert!(!store.is_empty());
    let posts: &Vec<i32> = store.get("posts").expect("posts should exist");
    assert_eq!(posts, &vec![42, 43, 44]);
}

#[test]
fn relation_store_get_missing_returns_none() {
    let store = RelationStore::new();
    let result: Option<&Vec<i32>> = store.get("nope");
    assert!(result.is_none());
}

// Minimal struct for testing — not a real Model
struct SimpleModel {
    id: i32,
    name: String,
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test with_relations_test`
Expected: FAIL — `RelationStore`, `WithRelations::bare` don't exist

**Step 3: Implement RelationStore and WithRelations**

Add to `sntl/src/core/relation.rs`:

```rust
use std::any::Any;
use std::collections::HashMap;
use std::ops::Deref;

/// Type-erased storage for loaded relation data.
///
/// Keyed by relation name. Stores pre-decoded Rust values as `Box<dyn Any>`.
/// Decode happens at Include execution time, not at accessor time.
pub struct RelationStore {
    data: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
}

impl RelationStore {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Store pre-decoded relation data.
    pub fn insert_decoded<T: Any + Send + Sync>(&mut self, name: &'static str, data: T) {
        self.data.insert(name, Box::new(data));
    }

    /// Retrieve typed relation data by name.
    pub fn get<T: Any>(&self, name: &str) -> Option<&T> {
        self.data.get(name)?.downcast_ref::<T>()
    }
}

impl Default for RelationStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper that pairs a model with its loaded relation data.
///
/// `M` = model type, `State` = tuple of Loaded/Unloaded per relation.
/// Deref to `M` for transparent field access.
pub struct WithRelations<M, State = ()> {
    model: M,
    relations: RelationStore,
    _state: PhantomData<State>,
}

impl<M, S> WithRelations<M, S> {
    pub fn new(model: M, relations: RelationStore) -> Self {
        Self { model, relations, _state: PhantomData }
    }

    pub fn into_inner(self) -> M {
        self.model
    }

    pub fn relations(&self) -> &RelationStore {
        &self.relations
    }
}

impl<M> WithRelations<M, ()> {
    /// Create a bare WithRelations with no loaded relations.
    pub fn bare(model: M) -> Self {
        Self::new(model, RelationStore::new())
    }
}

impl<M, S> Deref for WithRelations<M, S> {
    type Target = M;
    fn deref(&self) -> &M {
        &self.model
    }
}
```

**NOTE:** `WithRelations` uses `()` as default State. Models with 0 relations use `()`. Models with 1 relation use `(Unloaded,)`. Models with 2 use `(Unloaded, Unloaded)`. The comma in `(Unloaded,)` makes it a tuple not parens.

**Step 4: Add re-exports**

In `sntl/src/core/mod.rs`, add to existing re-exports:
```rust
pub use relation::{RelationStore, WithRelations};
```

In `sntl/src/core/prelude.rs`:
```rust
pub use crate::core::relation::{WithRelations, Loaded, Unloaded};
```

**Step 5: Run tests**

Run: `cargo test --package sntl --test with_relations_test`
Expected: PASS

**Step 6: Commit**

```bash
git add sntl/src/core/relation.rs sntl/src/core/mod.rs sntl/src/core/prelude.rs sntl/tests/with_relations_test.rs
git commit -m "feat: add WithRelations wrapper and RelationStore"
```

---

### Task 2: Add `RelationLoaded` trait with diagnostic

**Files:**
- Modify: `sntl/src/core/relation.rs`
- Test: `sntl/tests/with_relations_test.rs`
- Test: `sntl/tests/compile_fail/unloaded_relation_access.rs`

**Step 1: Write failing test for accessor via trait**

Add to `sntl/tests/with_relations_test.rs`:

```rust
use sntl::core::relation::RelationLoaded;

// Marker type for a test relation
struct TestPosts;

// Manual impl — in production, macro generates this
impl RelationLoaded<TestPosts> for WithRelations<SimpleModel, (Loaded,)> {
    type Output = Vec<i32>;
    fn get_relation(&self) -> &Vec<i32> {
        self.relations().get::<Vec<i32>>("posts").expect("posts loaded")
    }
}

#[test]
fn relation_loaded_trait_gates_access() {
    let mut store = RelationStore::new();
    store.insert_decoded("posts", vec![1i32, 2, 3]);
    let wr: WithRelations<SimpleModel, (Loaded,)> = WithRelations::new(
        SimpleModel { id: 1, name: "test".into() },
        store,
    );
    let posts: &Vec<i32> = wr.get_relation();
    assert_eq!(posts, &vec![1, 2, 3]);
}
```

**Step 2: Write compile-fail test for unloaded access**

Create `sntl/tests/compile_fail/unloaded_relation_access.rs`:

```rust
use sntl::core::relation::*;

struct FakeModel;
struct FakePosts;

// Only impl for Loaded state — Unloaded should fail
impl RelationLoaded<FakePosts> for WithRelations<FakeModel, (Loaded,)> {
    type Output = Vec<i32>;
    fn get_relation(&self) -> &Vec<i32> { todo!() }
}

fn main() {
    let wr: WithRelations<FakeModel, (Unloaded,)> = WithRelations::bare(FakeModel);
    // This should fail — Unloaded state doesn't impl RelationLoaded
    let _ = RelationLoaded::<FakePosts>::get_relation(&wr);
}
```

Create corresponding `sntl/tests/compile_fail/unloaded_relation_access.stderr` after first test run.

**Step 3: Implement RelationLoaded trait**

Add to `sntl/src/core/relation.rs`:

```rust
/// Trait gating access to relation data. Only implemented when the relation
/// is in `Loaded` state. Attempting to access an unloaded relation produces
/// a compile error with a helpful diagnostic message.
#[diagnostic::on_unimplemented(
    message = "relation `{Rel}` was not included in the query",
    label = "call .Include() to load this relation before accessing it"
)]
pub trait RelationLoaded<Rel> {
    type Output: ?Sized;
    fn get_relation(&self) -> &Self::Output;
}
```

**Step 4: Run tests**

Run: `cargo test --package sntl --test with_relations_test`
Expected: PASS

Run: `cargo test --package sntl --test compile_fail_test`
Expected: PASS (after updating .stderr file)

**Step 5: Commit**

```bash
git add sntl/src/core/relation.rs sntl/tests/with_relations_test.rs sntl/tests/compile_fail/
git commit -m "feat: add RelationLoaded trait with diagnostic::on_unimplemented"
```

---

### Task 3: Add `RelationInclude` typed marker and `IncludeTransition` trait

**Files:**
- Modify: `sntl/src/core/relation.rs`
- Test: `sntl/tests/include_transition_test.rs`

**Step 1: Write failing tests**

Create `sntl/tests/include_transition_test.rs`:

```rust
use sntl::core::relation::*;

struct User;
struct UserPosts;
struct UserProfile;

// Manual transition impls (macro generates these in production)
impl<B> IncludeTransition<User, (Unloaded, B), UserPosts> for () {
    type Next = (Loaded, B);
}
impl<A> IncludeTransition<User, (A, Unloaded), UserProfile> for () {
    type Next = (A, Loaded);
}

#[test]
fn include_transition_compiles() {
    // This test passes if it compiles — verifies the trait + impls work
    fn assert_transition<M, S, Rel, N>()
    where (): IncludeTransition<M, S, Rel, Next = N> {}

    assert_transition::<User, (Unloaded, Unloaded), UserPosts, (Loaded, Unloaded)>();
    assert_transition::<User, (Unloaded, Unloaded), UserProfile, (Unloaded, Loaded)>();
    // Including posts when profile already loaded
    assert_transition::<User, (Unloaded, Loaded), UserPosts, (Loaded, Loaded)>();
}

#[test]
fn relation_include_holds_spec() {
    let inc: RelationInclude<User, UserPosts> = RelationInclude::new(
        RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany)
    );
    assert_eq!(inc.spec().name(), "posts");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test include_transition_test`
Expected: FAIL — `IncludeTransition`, `RelationInclude` don't exist

**Step 3: Implement**

Add to `sntl/src/core/relation.rs`:

```rust
/// Trait for compile-time state transitions when including a relation.
///
/// `M` = model, `Current` = current state tuple, `Rel` = relation marker.
/// Macro generates impls that flip the Rel's position from Unloaded → Loaded.
pub trait IncludeTransition<M, Current, Rel> {
    type Next;
}

/// Typed relation include marker — carries both compile-time type info
/// and runtime RelationSpec for query execution.
pub struct RelationInclude<M, Rel> {
    spec: RelationSpec,
    _marker: PhantomData<(M, Rel)>,
}

impl<M, Rel> RelationInclude<M, Rel> {
    pub fn new(spec: RelationSpec) -> Self {
        Self { spec, _marker: PhantomData }
    }

    pub fn spec(&self) -> &RelationSpec {
        &self.spec
    }

    pub fn into_spec(self) -> RelationSpec {
        self.spec
    }
}
```

**Step 4: Run tests**

Run: `cargo test --package sntl --test include_transition_test`
Expected: PASS

**Step 5: Commit**

```bash
git add sntl/src/core/relation.rs sntl/tests/include_transition_test.rs
git commit -m "feat: add IncludeTransition trait and RelationInclude typed marker"
```

---

### Task 4: Refactor `ModelQuery` to `ModelQuery<M>`

**Files:**
- Modify: `sntl/src/core/query/pascal.rs`
- Modify: `sntl-macros/src/relation/codegen.rs`
- Test: existing tests must still pass (backward compat)

**Step 1: Run existing tests to confirm baseline**

Run: `cargo test --workspace`
Expected: PASS

**Step 2: Add generic param with default**

Change `sntl/src/core/query/pascal.rs`:

```rust
use std::marker::PhantomData;

#[must_use = "query does nothing until .FetchAll() or .Build() is called"]
pub struct ModelQuery<M = ()> {
    inner: SelectQuery,
    _model: PhantomData<M>,
}

impl<M> ModelQuery<M> {
    pub fn from_table(table: &str) -> Self {
        Self {
            inner: SelectQuery::new(table),
            _model: PhantomData,
        }
    }

    pub fn from_select(select: SelectQuery) -> Self {
        Self { inner: select, _model: PhantomData }
    }

    // All existing methods unchanged — Where, OrderBy, Limit, Offset, Build,
    // FetchAll, FetchOne, FetchOptional, FetchStream — just add <M> to impl
}
```

**Step 3: Update macro codegen to emit `ModelQuery<Self>`**

In `sntl-macros/src/relation/codegen.rs`, change `generate_pascal_find_methods`:

```rust
// Change return type from ModelQuery to ModelQuery<Self>
pub fn Find() -> sntl::core::query::ModelQuery<Self> {
    sntl::core::query::ModelQuery::from_table(<Self as sntl::core::Model>::TABLE)
}

pub fn FindId(id: impl Into<sntl::core::Value>) -> sntl::core::query::ModelQuery<Self> {
    // ...same body
}
```

**Step 4: Run all existing tests**

Run: `cargo test --workspace`
Expected: PASS — `ModelQuery<M = ()>` default makes existing code backward compatible. Tests that use `ModelQuery` without turbofish continue to work.

**Step 5: Commit**

```bash
git add sntl/src/core/query/pascal.rs sntl-macros/src/relation/codegen.rs
git commit -m "refactor: add generic model param to ModelQuery (backward compatible)"
```

---

### Task 5: Add `IncludeQuery` builder

**Files:**
- Create: `sntl/src/core/query/include.rs`
- Modify: `sntl/src/core/query/mod.rs`
- Modify: `sntl/src/core/query/pascal.rs`
- Test: `sntl/tests/include_query_test.rs`

**Step 1: Write failing tests**

Create `sntl/tests/include_query_test.rs`:

```rust
use sntl::core::relation::*;
use sntl::core::query::IncludeQuery;

struct User;
struct Post;
struct UserPosts;

impl sntl::core::Model for User {
    const TABLE: &'static str = "users";
    const PRIMARY_KEY: &'static str = "id";
    fn columns() -> &'static [sntl::core::ModelColumn] { &[] }
}

// Transition: including posts flips position 0
impl<B> IncludeTransition<User, (Unloaded, B), UserPosts> for () {
    type Next = (Loaded, B);
}

#[test]
fn include_query_tracks_specs() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany);
    let q: IncludeQuery<User, (Unloaded,)> = IncludeQuery::from_table("users");
    // Include returns new typed query
    let q2 = q.include_rel::<UserPosts>(spec);
    let (sql, _params) = q2.Build();
    assert!(sql.contains("users"));
    assert_eq!(q2.included_specs().len(), 1);
}

#[test]
fn include_query_chains_where() {
    let q: IncludeQuery<User, (Unloaded,)> = IncludeQuery::from_table("users");
    let q = q.Where(sntl::core::Column::new("users", "active").eq(true));
    let (sql, _) = q.Build();
    assert!(sql.contains("active"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test include_query_test`
Expected: FAIL — `IncludeQuery` doesn't exist

**Step 3: Implement IncludeQuery**

Create `sntl/src/core/query/include.rs`:

```rust
use std::marker::PhantomData;
use crate::core::expr::{Expr, OrderExpr};
use crate::core::query::SelectQuery;
use crate::core::relation::{IncludeTransition, RelationInclude, RelationSpec};
use crate::core::types::Value;

/// Query builder that tracks included relations in the type system.
///
/// Each `.Include()` call transitions the State type parameter,
/// ensuring compile-time safety for relation access on the result.
#[must_use = "query does nothing until .FetchAll() or .Build() is called"]
pub struct IncludeQuery<M, State = ()> {
    inner: SelectQuery,
    includes: Vec<RelationSpec>,
    _marker: PhantomData<(M, State)>,
}

impl<M, S> IncludeQuery<M, S> {
    pub fn from_table(table: &str) -> Self {
        Self {
            inner: SelectQuery::new(table),
            includes: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn from_parts(select: SelectQuery, includes: Vec<RelationSpec>) -> Self {
        Self { inner: select, includes, _marker: PhantomData }
    }

    /// Add a relation include with compile-time state transition.
    pub fn include_rel<Rel>(self, spec: RelationSpec) -> IncludeQuery<M, <() as IncludeTransition<M, S, Rel>>::Next>
    where
        (): IncludeTransition<M, S, Rel>,
    {
        let mut includes = self.includes;
        includes.push(spec);
        IncludeQuery {
            inner: self.inner,
            includes,
            _marker: PhantomData,
        }
    }

    /// Type-safe Include using RelationInclude marker.
    #[allow(non_snake_case)]
    pub fn Include<Rel>(self, inc: RelationInclude<M, Rel>) -> IncludeQuery<M, <() as IncludeTransition<M, S, Rel>>::Next>
    where
        (): IncludeTransition<M, S, Rel>,
    {
        self.include_rel::<Rel>(inc.into_spec())
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
    pub fn Limit(mut self, n: u64) -> Self {
        self.inner = self.inner.limit(n);
        self
    }

    #[allow(non_snake_case)]
    pub fn Build(&self) -> (String, Vec<Value>) {
        self.inner.build()
    }

    pub fn included_specs(&self) -> &[RelationSpec] {
        &self.includes
    }

    pub fn into_parts(self) -> (SelectQuery, Vec<RelationSpec>) {
        (self.inner, self.includes)
    }
}
```

Add to `sntl/src/core/query/mod.rs`:
```rust
pub mod include;
pub use include::IncludeQuery;
```

**Step 4: Add `.Include()` on `ModelQuery<M>` that transitions to `IncludeQuery`**

Add to `sntl/src/core/query/pascal.rs`:

```rust
use crate::core::relation::{IncludeTransition, RelationInclude, RelationSpec};
use crate::core::query::IncludeQuery;

impl<M> ModelQuery<M> {
    /// Start including relations — transitions to IncludeQuery for type tracking.
    #[allow(non_snake_case)]
    pub fn Include<Rel>(self, inc: RelationInclude<M, Rel>) -> IncludeQuery<M, <() as IncludeTransition<M, (), Rel>>::Next>
    where
        (): IncludeTransition<M, (), Rel>,
    {
        IncludeQuery::from_parts(self.inner, Vec::new()).Include(inc)
    }
}
```

**NOTE:** The initial state for `ModelQuery<M>.Include()` is `()` because ModelQuery doesn't know the model's relation count yet. The `IncludeTransition` impl generated by macro defines what `()` transitions to. For a model with 2 relations, the first Include transitions `()` → `(Loaded, Unloaded)`, not `(Unloaded, Unloaded)` → `(Loaded, Unloaded)`.

**ALTERNATIVE:** The macro could generate an associated type `type BareState = (Unloaded, Unloaded)` on Model, and `ModelQuery<M>.Include()` starts from `M::BareState`. This is cleaner. Add to Model trait:

```rust
pub trait ModelRelations {
    type BareState;
}
```

Macro implements `impl ModelRelations for User { type BareState = (Unloaded, Unloaded); }`.

Then `ModelQuery<M>.Include()` transitions from `M::BareState`.

**Step 5: Run tests**

Run: `cargo test --package sntl --test include_query_test`
Expected: PASS

Run: `cargo test --workspace`
Expected: PASS

**Step 6: Commit**

```bash
git add sntl/src/core/query/ sntl/tests/include_query_test.rs
git commit -m "feat: add IncludeQuery builder with type-state transitions"
```

---

### Task 6: Add `FetchOne`/`FetchAll` execution to `IncludeQuery`

**Files:**
- Modify: `sntl/src/core/query/include.rs`
- Modify: `sntl/src/core/model.rs` (add `primary_key_value` to Model trait)
- Test: `sntl/tests/pg_include_test.rs` (integration, skip without DB)

**Step 1: Add `primary_key_value()` to Model trait**

In `sntl/src/core/model.rs`:

```rust
pub trait Model {
    // ...existing...

    /// Extract the primary key value from this model instance.
    fn primary_key_value(&self) -> Value;
}
```

Update macro codegen in `sntl-macros/src/model/codegen.rs` `generate_model_impl`:

```rust
fn primary_key_value(&self) -> sntl::core::Value {
    self.#pk_field_name.clone().into()
}
```

**Step 2: Add FetchOne/FetchAll to IncludeQuery**

In `sntl/src/core/query/include.rs`:

```rust
impl<M, S> IncludeQuery<M, S>
where M: Model
{
    /// Execute main query + batch load all included relations.
    #[allow(non_snake_case)]
    pub async fn FetchOne(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<WithRelations<M, S>>
    where M: /* from_row — needs a trait or fn pointer */
    {
        let (select, includes) = self.into_parts();
        let row = select.fetch_one(conn).await?;
        let model = M::from_row(&row)?;
        let pk = model.primary_key_value();

        let mut store = RelationStore::new();
        for spec in &includes {
            let (sql, params) = spec.build_batch_sql(&[pk.clone()]);
            let param_refs: Vec<&(dyn driver::ToSql + Sync)> =
                params.iter().map(|p| p as &(dyn driver::ToSql + Sync)).collect();
            let rows = conn.query(&sql, &param_refs).await?;
            store.insert_raw(spec.name(), rows);
        }

        Ok(WithRelations::new(model, store))
    }

    #[allow(non_snake_case)]
    pub async fn FetchAll(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<Vec<WithRelations<M, S>>>
    where M: /* from_row */
    {
        let (select, includes) = self.into_parts();
        let rows = select.fetch_all(conn).await?;
        let models: Vec<M> = rows.iter()
            .map(|r| M::from_row(r))
            .collect::<Result<_, _>>()?;

        if includes.is_empty() {
            return Ok(models.into_iter()
                .map(|m| WithRelations::new(m, RelationStore::new()))
                .collect());
        }

        let pks: Vec<Value> = models.iter().map(|m| m.primary_key_value()).collect();

        // One batch query per relation
        let mut relation_rows: HashMap<&str, Vec<driver::Row>> = HashMap::new();
        for spec in &includes {
            let (sql, params) = spec.build_batch_sql(&pks);
            let param_refs: Vec<&(dyn driver::ToSql + Sync)> =
                params.iter().map(|p| p as &(dyn driver::ToSql + Sync)).collect();
            let rows = conn.query(&sql, &param_refs).await?;
            relation_rows.insert(spec.name(), rows);
        }

        // Distribute rows to each model's store by FK
        // ... grouping logic depends on FK column extraction from rows

        todo!("Row distribution by FK — implement in this task")
    }
}
```

**NOTE on `from_row`:** Currently `from_row` is a generated inherent method, not a trait method. For `IncludeQuery` to call it generically, we need either:
1. Add `fn from_row(row: &Row) -> Result<Self>` to `Model` trait
2. Add a separate `FromRow` trait

Option 1 is simplest — add to `Model` trait. Macro already generates `from_row`, just move the signature to the trait.

**NOTE on RelationStore:** We need two modes:
- `insert_raw(name, Vec<Row>)` — store raw rows, decode later per accessor
- `insert_decoded(name, T)` — store pre-decoded data (from Task 1)

For integration with real DB, `insert_raw` is needed. The accessor methods generated by macro will call `store.get_raw("posts")` and decode via `Post::from_row()`.

**Step 3: Write integration test**

Create `sntl/tests/pg_include_test.rs`:

```rust
#[macro_use]
mod pg_helpers;

use sntl::prelude::*;

#[tokio::test]
async fn include_posts_returns_with_relations() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert test data
    conn.execute(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        &[&"Alice", &"alice@test.com"],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO posts (user_id, title, body) VALUES ($1, $2, $3)",
        &[&1i32, &"Post 1", &"Body 1"],
    ).await.unwrap();

    // This test validates the full Include flow compiles and executes
    // Exact API depends on macro codegen from Task 7
}
```

**Step 4: Run tests**

Run: `cargo test --workspace`
Expected: PASS (integration tests skip without DB)

**Step 5: Commit**

```bash
git add sntl/src/core/ sntl/tests/pg_include_test.rs
git commit -m "feat: add FetchOne/FetchAll execution for IncludeQuery"
```

---

### Task 7: Macro codegen — relation markers, transitions, accessors

**Files:**
- Modify: `sntl-macros/src/relation/codegen.rs`
- Modify: `sntl-macros/src/relation/ir.rs` (add index tracking)
- Test: `sntl/tests/include_e2e_test.rs`

**Step 1: Write failing end-to-end test**

Create `sntl/tests/include_e2e_test.rs` using the full macro-generated API:

```rust
use sntl::prelude::*;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
    #[sentinel(default)]
    pub active: bool,
    #[sentinel(default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Model)]
#[sentinel(table = "posts")]
pub struct Post {
    #[sentinel(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: String,
    #[sentinel(default)]
    pub published: bool,
    #[sentinel(default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[sentinel(relations)]
impl User {
    pub fn posts() -> HasMany<Post> {
        HasMany::new("user_id")
    }
}

#[test]
fn macro_generates_typed_include() {
    // User::Posts() returns RelationInclude<User, UserPosts>
    let _inc = User::Posts();
}

#[test]
fn macro_generates_type_aliases() {
    // These type aliases should exist
    let _: fn() -> UserBare = || todo!();
}

#[test]
fn include_query_type_transitions() {
    // This must compile — proves type-state chain works
    let _q = User::Find()
        .Include(User::Posts());
    // Return type should be IncludeQuery<User, (Loaded,)>
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package sntl --test include_e2e_test`
Expected: FAIL — `User::Posts()`, `UserBare`, `UserPosts` don't exist

**Step 3: Expand macro codegen**

In `sntl-macros/src/relation/codegen.rs`, add new generation functions:

```rust
pub fn generate_relations(ir: &RelationIR) -> TokenStream {
    let relation_consts = generate_relation_constants(ir);
    let pascal_methods = generate_pascal_find_methods(ir);
    let markers = generate_relation_markers(ir);
    let transitions = generate_include_transitions(ir);
    let typed_includes = generate_typed_include_methods(ir);
    let accessors = generate_relation_accessors(ir);
    let type_aliases = generate_type_aliases(ir);
    let bare_state = generate_bare_state(ir);

    quote! {
        #relation_consts
        #pascal_methods
        #markers
        #transitions
        #typed_includes
        #accessors
        #type_aliases
        #bare_state
    }
}

fn generate_relation_markers(ir: &RelationIR) -> TokenStream {
    // Generate: pub struct UserPosts; pub struct UserProfile;
    let markers: Vec<TokenStream> = ir.relations.iter().map(|rel| {
        let marker_name = format_ident!("{}{}", ir.model_name, pascal_case(&rel.fn_name.to_string()));
        quote! { pub struct #marker_name; }
    }).collect();
    quote! { #(#markers)* }
}

fn generate_include_transitions(ir: &RelationIR) -> TokenStream {
    // For each relation at index i, generate:
    // impl<..other generics..> IncludeTransition<Model, (..Unloaded at i..), Marker> for () {
    //     type Next = (..Loaded at i..);
    // }
    // Uses generic params for all OTHER positions
}

fn generate_typed_include_methods(ir: &RelationIR) -> TokenStream {
    // Generate: impl User { pub fn Posts() -> RelationInclude<User, UserPosts> { ... } }
}

fn generate_relation_accessors(ir: &RelationIR) -> TokenStream {
    // Generate: impl<B> WithRelations<User, (Loaded, B)> { pub fn posts() -> ... }
    // Uses get_many for HasMany, get_one for HasOne/BelongsTo
}

fn generate_type_aliases(ir: &RelationIR) -> TokenStream {
    // Generate: type UserBare = WithRelations<User, (Unloaded,)>;
    //           type UserWithPosts = WithRelations<User, (Loaded,)>;
    // For 2 relations: UserBare, UserWithPosts, UserWithProfile, UserFull
}

fn generate_bare_state(ir: &RelationIR) -> TokenStream {
    // Generate: impl ModelRelations for User { type BareState = (Unloaded, Unloaded); }
}
```

**IMPORTANT implementation details for `generate_include_transitions`:**

For a model with relations at indices 0..N, relation i gets:
```rust
impl<A, B, ..> IncludeTransition<Model, (A, .., Unloaded_at_i, .., Z), MarkerI> for () {
    type Next = (A, .., Loaded, .., Z);
}
```
Where all positions except i are generic params. This ensures N impls, not 2^N.

**Step 4: Run tests**

Run: `cargo test --package sntl --test include_e2e_test`
Expected: PASS

Run: `cargo test --workspace`
Expected: PASS

**Step 5: Commit**

```bash
git add sntl-macros/src/relation/ sntl/tests/include_e2e_test.rs
git commit -m "feat: macro codegen for relation markers, transitions, accessors, type aliases"
```

---

### Task 8: Integration test — full Include flow with live PG

**Files:**
- Modify: `sntl/tests/pg_include_test.rs`

**Step 1: Write full integration test**

```rust
#[tokio::test]
async fn include_fetches_related_data() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Seed data
    conn.execute("INSERT INTO users (name, email) VALUES ('Alice', 'alice@test.com')", &[]).await.unwrap();
    conn.execute("INSERT INTO posts (user_id, title, body) VALUES (1, 'Post 1', 'Body 1')", &[]).await.unwrap();
    conn.execute("INSERT INTO posts (user_id, title, body) VALUES (1, 'Post 2', 'Body 2')", &[]).await.unwrap();

    // Include query
    let user = User::FindId(1)
        .Include(User::Posts())
        .FetchOne(&mut conn)
        .await
        .unwrap();

    // Model fields via Deref
    assert_eq!(user.name, "Alice");

    // Relation accessor — compile-time guaranteed to be loaded
    let posts = user.posts();
    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].title, "Post 1");
}

#[tokio::test]
async fn include_fetch_all_batches_correctly() {
    let url = require_pg!();
    // ... seed 2 users with 2 posts each
    // Verify: exactly 2 queries (1 users + 1 posts), not 3

    let users = User::Find()
        .Include(User::Posts())
        .FetchAll(&mut conn)
        .await
        .unwrap();

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].posts().len(), 2);
    assert_eq!(users[1].posts().len(), 2);
}
```

**Step 2: Run integration tests**

Run: `DATABASE_URL=... cargo test --package sntl --test pg_include_test`
Expected: PASS

**Step 3: Commit**

```bash
git add sntl/tests/pg_include_test.rs
git commit -m "test: integration tests for Include flow with live PG"
```

---

### Task 9: Compile-fail tests for unloaded relation access

**Files:**
- Create: `sntl/tests/compile_fail/include_required.rs`
- Update: `sntl/tests/compile_fail_test.rs`

**Step 1: Write compile-fail test**

Create `sntl/tests/compile_fail/include_required.rs`:

```rust
use sntl::prelude::*;

// ... User + Post definitions with relations ...

fn main() {
    // This must NOT compile — posts() requires Loaded state
    let user: UserBare = todo!();
    let _ = user.posts();
}
```

Expected `.stderr`:
```
error: relation `UserPosts` was not included in the query
  --> tests/compile_fail/include_required.rs:XX:XX
   |
XX |     let _ = user.posts();
   |                  ^^^^^ call .Include() to load this relation before accessing it
```

**Step 2: Run compile-fail tests**

Run: `cargo test --package sntl --test compile_fail_test`
Expected: PASS after generating correct .stderr

**Step 3: Commit**

```bash
git add sntl/tests/compile_fail/
git commit -m "test: compile-fail tests for unloaded relation access"
```

---

### Task 10: Clippy, coverage, full test suite, PR

**Files:** All modified files

**Step 1: Format**

Run: `cargo fmt --all`

**Step 2: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Fix any warnings.

**Step 3: Full test suite**

Run: `cargo test --workspace`
Expected: All PASS

**Step 4: Coverage**

Run: `cargo llvm-cov --workspace --ignore-filename-regex '...' --fail-under-lines 100`
Add tests for any uncovered lines.

**Step 5: Integration tests**

Run: `DATABASE_URL=... cargo test --workspace`

**Step 6: Create PR**

```bash
gh pr create --title "feat: Phase 5B-1 — type-state pattern for compile-time relation safety" \
  --body "..."
```
