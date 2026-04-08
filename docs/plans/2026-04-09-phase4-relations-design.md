# Phase 4: Type-State Relations Design

**Date:** 2026-04-09
**Status:** Approved
**Goal:** Compile-time N+1 prevention via type-state pattern — accessing an unloaded relation is a compile error, not a runtime surprise.

---

## 1. Relation Declaration

Relations are declared in a separate `#[sentinel(relations)]` impl block, not as struct fields. Struct fields = DB columns only.

```rust
#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[sentinel(relations)]
impl User {
    pub fn posts() -> HasMany<Post> { HasMany::new("user_id") }
    pub fn profile() -> HasOne<Profile> { HasOne::new("user_id") }
}

#[sentinel(relations)]
impl Post {
    pub fn author() -> BelongsTo<User> { BelongsTo::new("user_id") }
}
```

**Relation types:** `HasMany<T>`, `HasOne<T>`, `BelongsTo<T>`

**Macro generates per relation:**
- Relation constant: `User::POSTS`, `User::PROFILE`, `Post::AUTHOR`
- Generic type parameter on model struct
- Gated accessor method

---

## 2. Type-State Generated Code

Per-relation generic parameter approach. Each relation gets its own `Loaded`/`Unloaded` generic.

```rust
pub struct Loaded;
pub struct Unloaded;

pub struct User<Posts = Unloaded, Profile = Unloaded> {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    __posts: Option<Vec<Post>>,
    __profile: Option<Box<Profile>>,
    __state: PhantomData<(Posts, Profile)>,
}

// Type aliases
pub type UserBare = User<Unloaded, Unloaded>;
pub type UserWithPosts = User<Loaded, Unloaded>;
pub type UserWithProfile = User<Unloaded, Loaded>;
pub type UserFull = User<Loaded, Loaded>;

// posts() only when Posts = Loaded
impl<Profile> User<Loaded, Profile> {
    pub fn posts(&self) -> &[Post] {
        self.__posts.as_deref().unwrap()
    }
}

// profile() only when Profile = Loaded
impl<Posts> User<Posts, Loaded> {
    pub fn profile(&self) -> &Profile {
        self.__profile.as_ref().unwrap()
    }
}
```

**Error messages via `#[diagnostic::on_unimplemented]`:**
```
error[E0599]: no method named `posts` found for `User<Unloaded, Unloaded>`
  help: relation `posts` was not included — call .Include(User::POSTS)
```

**Internal storage:** `__posts: Option<Vec<Post>>` (HasMany), `__profile: Option<Box<Profile>>` (HasOne). Double-underscore prefix to avoid field name collision. PhantomData has zero runtime cost.

---

## 3. Query API (PascalCase)

```rust
// Basic
let users = User::Find().FetchAll(&mut conn).await?;
let user = User::FindId(id).FetchOne(&mut conn).await?;

// Include (type transitions)
let user = User::FindId(id)
    .Include(User::POSTS)
    .FetchOne(&mut conn).await?;
// type: User<Loaded, Unloaded>

// Multiple includes
let user = User::FindId(id)
    .Include(User::POSTS)
    .Include(User::PROFILE)
    .FetchOne(&mut conn).await?;
// type: User<Loaded, Loaded>

// Filtered includes
let user = User::FindId(id)
    .Include(User::POSTS
        .Filter(Post::PUBLISHED.eq(true))
        .OrderBy(Post::CREATED_AT.desc())
        .Limit(5))
    .FetchOne(&mut conn).await?;

// Chain with Where/OrderBy/Limit
let users = User::Find()
    .Where(User::EMAIL.like("%@corp.com"))
    .OrderBy(User::NAME.asc())
    .Limit(20)
    .Include(User::POSTS)
    .FetchAll(&mut conn).await?;
```

**PascalCase methods:** `Find`, `FindId`, `FetchOne`, `FetchAll`, `FetchOptional`, `FetchStream`, `Where`, `OrderBy`, `Limit`, `Offset`, `Include`, `Filter`

**SQL generation — automatic WHERE IN batching:**
- N includes = N+1 queries (guaranteed, documented)
- `Include(User::POSTS)` → `SELECT * FROM posts WHERE user_id IN ($1, $2, ...)`
- N+1 structurally impossible

---

## 4. Explicit Load + Batch Load

```rust
// Load later (consumes self, returns upgraded type)
let user = User::FindId(id).FetchOne(&mut conn).await?;
let user_with_posts = user.Load(User::POSTS, &mut conn).await?;
// type: User<Loaded, Unloaded>

// Batch load for collections
let users = User::Find().FetchAll(&mut conn).await?;
let users_with_posts = User::BatchLoad(User::POSTS, users, &mut conn).await?;
// type: Vec<User<Loaded, Unloaded>>
// exactly 1 query: SELECT * FROM posts WHERE user_id IN (...)

// Nested includes
let user = User::FindId(id)
    .Include(User::POSTS.Include(Post::AUTHOR))
    .FetchOne(&mut conn).await?;
// user.posts()[0].author() → &User
// 3 queries total
```

---

## 5. Edge Cases

### Circular relations — Depth limit = 3

```rust
// ✅ Depth 1-3 allowed
User::POSTS.Include(Post::AUTHOR)                           // depth 2
User::POSTS.Include(Post::COMMENTS.Include(Comment::AUTHOR)) // depth 3

// ❌ Depth 4+ compile error
// "nested include depth exceeds maximum (3). Use .Load() for deeper relations"
```

Implementation: `IncludeSpec<R, const DEPTH: usize>` with const generic depth counter.

### Partial + Include compose

Partial types inherit parent's relation generics:

```rust
#[derive(Partial)]
#[partial(model = User, fields = [id, email])]
pub struct UserEmail;

// Generates: pub struct UserEmail<Posts = Unloaded, Profile = Unloaded> { ... }

let users = User::Find()
    .SelectAs::<UserEmail>()
    .Include(User::POSTS)
    .FetchAll(&mut conn).await?;
// type: Vec<UserEmail<Loaded, Unloaded>>
// SQL: SELECT id, email FROM users (partial)
//    + SELECT * FROM posts WHERE user_id IN (...) (full relation)
```

### Self-referential relations

No special case — just a relation to the same model. Depth limit prevents infinite recursion.

```rust
#[sentinel(relations)]
impl User {
    pub fn subordinates() -> HasMany<User> { HasMany::new("manager_id") }
    pub fn manager() -> BelongsTo<User> { BelongsTo::new("manager_id") }
}
```

### Serde serialization (future)

Loaded relations serialize, Unloaded relations omitted from JSON. Custom Serialize impl generated by macro.

### PooledConnection

No changes needed — Deref handles it. All `&mut Connection` methods work with `&mut PooledConnection`.

### Relation count without loading (future)

```rust
let count = user.PostsCount(&mut conn).await?;
// SQL: SELECT COUNT(*) FROM posts WHERE user_id = $1
```

### Migration IR (design now, implement Phase 7)

```rust
pub struct RelationIR {
    pub name: String,
    pub kind: RelationKind,      // HasMany, HasOne, BelongsTo
    pub target_model: String,
    pub foreign_key: String,
    pub target_table: String,
}
```

---

## 6. Testing Strategy

| Layer | What | How |
|---|---|---|
| Unit | Relation declaration parsing | trybuild compile_fail |
| Unit | SQL generation for includes | Assert SQL + params |
| Unit | Type-state transitions | Verify generic params |
| Compile-fail | Access unloaded relation | trybuild `.stderr` |
| Compile-fail | Missing foreign key | trybuild error |
| Integration | Include loads correct data | pg_* tests |
| Integration | BatchLoad WHERE IN | pg_* tests |
| Integration | Filtered includes | pg_* tests |
| Integration | Nested includes | pg_* tests |

---

## 7. NOT in Phase 4

| Item | Phase |
|---|---|
| `#[reducer]` transactions | 5 |
| `sql!()` macro / sqlparser-rs | 6 |
| Many-to-many (`HasManyThrough`) | 4.5 |
| schema.sntl + `cargo sntl init` | 7 |
| MCP folder for AI tooling | 7 |
| Query scopes / default scopes | 5 |
| Optimistic locking | 5 |
