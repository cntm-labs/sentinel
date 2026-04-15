# Phase 5B: Type-State Pattern — Design Document

> **Goal:** Compile-time N+1 prevention through type-state gated relation access.
> Accessing a relation that wasn't `.Include()`d is a compile error, not a runtime panic.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Scope | Nested includes, 2 sub-phases | 5B-1 flat, 5B-2 nested |
| Generic strategy | Per-relation generic param | Precise types, clear error messages |
| Struct strategy | Wrapper `WithRelations<M, State>` | No mutation of user's struct |
| Include API | Builder with type accumulation | Compile-time safety throughout chain |
| Error messages | Trait + `#[diagnostic::on_unimplemented]` | Custom errors guide user to fix |

## Architecture: Library Core + Macro Glue

Library (`sntl/src/core/`) provides generic infrastructure. Macro generates per-model glue code. This keeps core logic testable without proc macros.

---

## Core Types (Library Layer)

### WithRelations Wrapper

```rust
pub struct WithRelations<M, State = ()> {
    model: M,
    relations: RelationStore,
    _state: PhantomData<State>,
}

impl<M, S> Deref for WithRelations<M, S> {
    type Target = M;
    fn deref(&self) -> &M { &self.model }
}
```

- `M` = model type (User, Post)
- `State` = tuple of `Loaded`/`Unloaded` per relation, position = relation index
- `Deref` enables `user.name` to access model fields transparently

### RelationStore

```rust
pub struct RelationStore {
    data: HashMap<&'static str, Vec<driver::Row>>,
}

impl RelationStore {
    pub fn get_many<T: FromRow>(&self, name: &str) -> Vec<T> { ... }
    pub fn get_one<T: FromRow>(&self, name: &str) -> T { ... }
}
```

Stores raw rows keyed by relation name. Decodes lazily when accessor is called.

---

## Relation Access Trait + Diagnostic

```rust
#[diagnostic::on_unimplemented(
    message = "relation `{Rel}` was not included",
    label = "call .Include(Model::RELATION_NAME) to load this relation"
)]
pub trait RelationLoaded<Rel> {
    type Output;
    fn get_relation(&self) -> &Self::Output;
}
```

Macro generates N impls per model (not 2^N) using generic bounds per-position:

```rust
// 2 relations = 2 impls, not 4
impl<B> WithRelations<User, (Loaded, B)> {
    pub fn posts(&self) -> Vec<Post> { ... }
}
impl<A> WithRelations<User, (A, Loaded)> {
    pub fn profile(&self) -> Profile { ... }
}
```

Error when accessing unloaded relation:
```
error: relation `UserPosts` was not included
  --> src/main.rs:10:5
   |
10 |     user.posts()
   |          ^^^^^ call .Include(User::POSTS) to load this relation
```

---

## IncludeQuery Builder (Type Accumulation)

### Typed Include Markers

`.Include()` takes typed markers instead of runtime `RelationSpec`:

```rust
pub struct RelationInclude<M, Rel> {
    spec: RelationSpec,
    _marker: PhantomData<(M, Rel)>,
}
```

Macro generates typed constructors:
```rust
impl User {
    pub fn Posts() -> RelationInclude<User, UserPosts> { ... }
    pub fn Profile() -> RelationInclude<User, UserProfile> { ... }
}
```

### IncludeQuery

```rust
pub struct IncludeQuery<M, State = ()> {
    select: SelectQuery,
    includes: Vec<RelationSpec>,
    _marker: PhantomData<(M, State)>,
}
```

### State Transitions via Trait

```rust
pub trait IncludeTransition<M, CurrentState, Rel> {
    type NextState;
}

// Macro generates:
impl<B> IncludeTransition for (User, (Unloaded, B), UserPosts) {
    type Output = (Loaded, B);
}
impl<A> IncludeTransition for (User, (A, Unloaded), UserProfile) {
    type Output = (A, Loaded);
}
```

### ModelQuery Refactor

```rust
pub struct ModelQuery<M = ()> {
    inner: SelectQuery,
    _model: PhantomData<M>,
}
```

`M = ()` default makes existing code backward compatible. First `.Include()` call transitions `ModelQuery<User>` → `IncludeQuery<User, State>`.

### Usage Flow

```rust
User::FindId(1)                       // ModelQuery<User>
    .Include(User::Posts())            // IncludeQuery<User, (Loaded, Unloaded)>
    .Include(User::Profile())          // IncludeQuery<User, (Loaded, Loaded)>
    .Where(User::ACTIVE.eq(true))      // still chainable
    .FetchOne(&mut conn).await         // WithRelations<User, (Loaded, Loaded)>
```

---

## Execution Flow

### FetchOne

```
1. SELECT * FROM users WHERE id = $1           → User row
2. SELECT * FROM posts WHERE user_id IN ($1)   → Post rows
3. SELECT * FROM profiles WHERE user_id IN ($1) → Profile row
→ WithRelations<User, (Loaded, Loaded)>
```

### FetchAll (Batch Optimized)

```
1. SELECT * FROM users WHERE active = true     → N user rows
2. Collect all user PKs
3. SELECT * FROM posts WHERE user_id IN ($1..$N)    → all posts, 1 query
4. SELECT * FROM profiles WHERE user_id IN ($1..$N) → all profiles, 1 query
5. Group by FK, distribute to each parent's RelationStore
→ Vec<WithRelations<User, (Loaded, Loaded)>>
```

Exactly R+1 queries (1 main + R relations), regardless of parent row count.

---

## What Macro Generates (5B-1)

Given:
```rust
#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key)]
    pub id: i32,
    pub name: String,
}

#[sentinel(relations)]
impl User {
    pub fn posts() -> HasMany<Post> { HasMany::new("user_id") }
    pub fn profile() -> HasOne<Profile> { HasOne::new("user_id") }
}
```

Macro emits:
1. **Relation marker types:** `UserPosts`, `UserProfile`
2. **Typed include constructors:** `User::Posts()`, `User::Profile()`
3. **IncludeTransition impls:** N impls (one per relation)
4. **Accessor methods:** gated by `(Loaded, B)` / `(A, Loaded)` patterns
5. **Type aliases:** `UserBare`, `UserWithPosts`, `UserWithProfile`, `UserFull`
6. **Existing `User::POSTS` const preserved** for batch SQL generation

---

## Sub-phases

### 5B-1: Flat Includes
- All library core types
- Macro glue for 1-level include
- `FetchOne` + `FetchAll` execution
- Compile-fail tests for unloaded access
- Integration tests with live PG

### 5B-2: Nested Includes
- `RelationInclude` gains `.Include()` method for chaining
- Recursive batch loading (main → children → grandchildren)
- Const generic depth counter, cap at 3
- Child models become `Vec<WithRelations<Post, ...>>` instead of `Vec<Post>`

---

## Backward Compatibility

| Existing API | Impact |
|---|---|
| `User` struct | No change |
| `ModelQuery` | Gains `<M = ()>` — default preserves all existing usage |
| `User::POSTS` const | Preserved alongside new `User::Posts()` |
| `#[derive(Partial)]` | No change — operates on plain struct |
| `from_row()` | No change — decodes into plain model |
| Query builder chain | `.Where()`, `.OrderBy()` etc. work on both `ModelQuery` and `IncludeQuery` |
