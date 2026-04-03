# Sentinel ORM — Design Document

> **Sentinel** — A compile-time guarded Rust ORM for PostgreSQL.
>
> *"Your data's guardian — from compile to production"*

**Date:** 2026-04-03
**Status:** Approved
**Author:** mrbt + Claude

---

## Overview

Sentinel is a standalone Rust ORM crate for PostgreSQL, designed around three principles:

1. **Guard at compile-time** — N+1, over-fetching, unsafe relation access caught at compile, not runtime
2. **Zero surprise** — no lazy loading, no hidden queries, every DB call explicit
3. **No cliff** — from simple CRUD to complex CTEs, always type-safe, always parameterized

### Project Scope

- **v1:** Standalone crate on crates.io — PostgreSQL only
- **sentinel-driver:** Custom PG wire protocol driver (separate repo) — replaces sqlx
- **Layer 2 (future):** Realtime platform built on Sentinel (like Clerk uses NextAuth as core)

### Tech Stack

- **Language:** Rust (stable)
- **Database:** PostgreSQL (only)
- **Async:** tokio
- **TLS:** rustls (no OpenSSL dependency)
- **Driver:** sentinel-driver (custom PG wire protocol)

---

## Architecture

```
sentinel (workspace)
|- sentinel-core       — Model trait, QueryBuilder, Transaction, Relations
|- sentinel-macros     — derive(Model), derive(Partial), reducer
|- sentinel-migrate    — Schema diff, migration generation
|- sentinel-cli        — CLI tool

sentinel-driver (separate repo/crate)
|- protocol/           — PG wire protocol v3 parser
|- pool/               — Async connection pool
|- auth/               — SCRAM-SHA-256 (correct SASLprep)
|- pipeline/           — PG pipeline mode
|- copy/               — COPY protocol for bulk ops
|- notify/             — LISTEN/NOTIFY engine
|- types/              — PG type encoding/decoding (binary)
|- tls/                — rustls integration
```

### Why NOT sqlx?

sqlx has 100 open issues including 30 bugs. Critical problems:

1. Compile-time macros require live DB connection — builds hang 2+ hours if DB unreachable
2. 3 soundness bugs in SQLite — unsafe code reaching safe APIs
3. SCRAM auth bug — does not SASLprep passwords
4. Multi-database abstraction tax — PG features (pipeline, COPY) are second-class
5. cargo sqlx prepare broken — requires cargo clean to work

sentinel-driver is PG-only = optimized deep, not wide.

---

## Developer Experience

### Model Definition

```rust
use sentinel::prelude::*;

#[derive(Model)]
#[model(table = "users", doc = "Core user identity")]
pub struct User {
    #[model(primary, default = "gen_random_uuid()")]
    pub id: Uuid,

    #[model(unique, doc = "Login email, verified via OAuth")]
    pub email: String,

    pub name: Option<String>,

    #[model(default = "now()")]
    pub created_at: DateTime<Utc>,

    #[model(has_many)]
    pub posts: Relation<Vec<Post>>,
}

#[derive(Model)]
#[model(table = "posts", audit = true)]
pub struct Post {
    #[model(primary)]
    pub id: Uuid,
    pub title: String,
    pub body: String,
    #[model(belongs_to = "User")]
    pub author: Relation<User>,
    pub author_id: Uuid,
}
```

### CRUD Operations

```rust
// Create
let user = User::create(NewUser {
    email: "alice@example.com".into(),
    name: Some("Alice".into()),
}).exec(&db).await?;

// Read
let user = User::find_by_id(id).one(&db).await?;
let users = User::find()
    .where_(User::EMAIL.ends_with("@example.com"))
    .order_by(User::CREATED_AT.desc())
    .limit(20)
    .all(&db).await?;

// Update
let user = User::update(id)
    .set(User::NAME, Some("Alice Smith"))
    .exec(&db).await?;

// Delete
User::delete(id).exec(&db).await?;
```

### Relations (N+1 Safe via Type-State Pattern)

```rust
// Compile error if relation not included
let user = User::find_by_id(id).one(&db).await?;
user.posts()  // compile error: User<Bare> has no method posts

// Correct — include generates single WHERE IN query
let user = User::find_by_id(id)
    .include(User::POSTS)
    .one(&db).await?;
user.posts()  // Vec<Post>

// Nested include
let user = User::find_by_id(id)
    .include(User::POSTS.include(Post::COMMENTS))
    .one(&db).await?;

// Explicit load later (clear that DB call happens)
let user = User::find_by_id(id).one(&db).await?;
let posts = user.load(User::POSTS, &db).await?;

// Batch load for collections (prevents N+1)
let users = User::find().all(&db).await?;
let users_with_posts = User::batch_load(User::POSTS, &users, &db).await?;
```

Type-state implementation:
```rust
struct User<State = Bare> { ... }
struct Bare;
struct WithPosts;

impl User<WithPosts> {
    pub fn posts(&self) -> &[Post] { ... }  // only available when included
}
```

### Partial Select (No Over-fetching)

```rust
#[derive(Partial)]
#[partial(model = User, fields = [id, email])]
pub struct UserEmail;

let users = User::find()
    .select_as::<UserEmail>()
    .all(&db).await?;
// type: Vec<UserEmail> — only id + email fetched from DB
```

### Transactions

```rust
// Auto-transaction (reducer pattern, inspired by SpacetimeDB)
#[reducer]
async fn transfer(db: &Db, from: Uuid, to: Uuid, amount: Decimal) -> Result<()> {
    let mut src = Account::find_by_id(from).for_update().one(db).await?;
    let mut dst = Account::find_by_id(to).for_update().one(db).await?;
    // ORM reorders locks by ID -> deadlock prevention
    src.balance -= amount;
    dst.balance += amount;
    src.save(db).await?;
    dst.save(db).await?;
    Ok(())  // auto-COMMIT on Ok, auto-ROLLBACK on Err
}

// Manual transaction with savepoints
db.transaction(|tx| async {
    let user = User::create(new_user).exec(&tx).await?;
    tx.savepoint(|sp| async {
        Notification::send(user.id).exec(&sp).await?;
        Ok(())
    }).await.ok();
    Ok(user)
}).await?;
```

### Query Layers (No Cliff)

```rust
// Layer 1: Fluent API (80% of queries)
User::find().where_(User::AGE.gt(25)).all(&db).await?;

// Layer 2: Advanced builder (JOINs, aggregates)
User::find()
    .join(User::POSTS)
    .group_by(User::ID)
    .select((User::NAME, Post::ID.count().as_("post_count")))
    .having(Post::ID.count().gt(5))
    .all(&db).await?;

// Layer 3: Typed raw SQL (CTEs, window functions)
db.query_as::<UserRank>("WITH ranked AS (...) SELECT * FROM ranked WHERE rank <= $1")
    .bind(3).all().await?;

// Layer 4: Dynamic builder (still parameterized)
let mut q = QueryBuilder::select_from("users");
q.column("id").column("email");
q.where_eq("active", true);
q.fetch_all::<User>(&db).await?;
```

---

## Safety and Security

| Feature | Implementation |
|---------|---------------|
| SQL injection | Parameterized at every layer, including dynamic builder |
| N+1 prevention | Type-state pattern, compile error if relation not included |
| Over-fetching | Strict select mode, compile error if fields not selected |
| Deadlock prevention | Auto-reorder locks by ID in transactions |
| Transaction safety | RAII, drop = rollback, cannot forget to commit |
| Auth | SCRAM-SHA-256 with correct SASLprep |
| TLS | rustls, no OpenSSL |
| Audit trail | model(audit = true) auto-logs every change |
| Unsafe code | Zero unsafe in sentinel-core, minimal in driver |

---

## Migration System

Migrations detected from Rust struct changes (snapshot comparison).
Migration files are plain SQL — DBA can review and edit.
Migration history tracked in _sentinel_migrations table.
Doc attributes enforced: CLI warns if new fields lack doc attribute.

CLI commands:
- sentinel migrate create <name> — detect struct diff, generate SQL
- sentinel migrate run — apply pending
- sentinel migrate rollback — rollback last
- sentinel migrate plan — preview pending changes
- sentinel migrate history — show timeline
- sentinel migrate rename <old> <new> — safe rename with migration

---

## Performance Targets

| Metric | sqlx (baseline) | sentinel-driver (target) |
|--------|-----------------|-------------------------|
| Simple SELECT | 75K q/s | 90K+ q/s |
| Batch 100 queries | 3K batch/s | 15K+ batch/s |
| Bulk INSERT 10K rows | 50K rows/s | 500K+ rows/s |
| Pool checkout | 0.8 us | <0.5 us |
| Stmt cache hit | ~90% | ~99% |

### Performance Techniques

1. Binary encoding by default (+15-40%)
2. Auto-pipeline mode (+5-20x for batch)
3. Zero-copy parsing with bytes::Bytes (+10-30%)
4. Single-task architecture (-1-2 us/query)
5. COPY protocol for bulk ops (10-50x faster)
6. unnest() batching for batch inserts
7. Two-tier stmt cache: HashMap (known) + LRU-256 (ad-hoc)

---

## CLI and Tooling

```
sentinel init                    — Initialize project
sentinel migrate create <name>   — Generate migration from struct diff
sentinel migrate run             — Apply pending migrations
sentinel migrate rollback        — Rollback last migration
sentinel health check            — Data validation against constraints
sentinel health locks            — Show active locks/deadlocks
sentinel perf report             — Query performance summary
sentinel explain <query>         — Human-readable query plan
sentinel docs generate           — Generate schema documentation
```

---

## v1 vs Layer 2 Scope

### v1 (ORM Core)
- Schema = Rust struct with derive macros
- Migration auto-detect from struct diff
- Doc attributes with CLI warnings
- 4-layer query builder (no SELECT star)
- Audit trail
- Deadlock prevention
- Prepared statement cache
- Connection pooling
- Schema diff CLI
- Query explain
- Performance tracking

### Layer 2 (Future Platform)
- Visual Studio (web UI)
- AI query assistant
- AI schema explainer
- Data cleaning tools (80/20 rule)
- Live queries (realtime via LISTEN/NOTIFY)
- Cross-database catalog
- Full documentation generator
- Experiment tracking
- Data audit and anomaly detection

---

## Inspiration Sources

| Source | What we take | What we avoid |
|--------|-------------|---------------|
| Prisma | Schema-first DX, explicit include, migration as SQL | Cross-language complexity, DSL lock-in |
| SpacetimeDB | Reducer pattern, subscription model, diff-based updates | WASM-only, single-writer limitation |
| SurrealDB | Record links, LIVE SELECT, permissions, embedded mode | Custom query language, multi-model complexity |
| Diesel | Compile-time safety, type system | Verbose DSL, poor async, schema.rs sync |
| sqlx | Async-first, prepared stmt cache | Live DB at compile, soundness bugs, multi-DB tax |
