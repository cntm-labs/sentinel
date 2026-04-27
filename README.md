<div align="center">

# sentinel

**Compile-time guarded ORM for PostgreSQL — your data's guardian from compile to production.**

[![CI](https://github.com/cntm-labs/sentinel/actions/workflows/ci.yml/badge.svg)](https://github.com/cntm-labs/sentinel/actions/workflows/ci.yml)
[![PostgreSQL Integration](https://github.com/cntm-labs/sentinel/actions/workflows/postgresql.yml/badge.svg)](https://github.com/cntm-labs/sentinel/actions/workflows/postgresql.yml)
[![codecov](https://codecov.io/gh/cntm-labs/sentinel/branch/main/graph/badge.svg)](https://codecov.io/gh/cntm-labs/sentinel)
[![Security](https://github.com/cntm-labs/sentinel/actions/workflows/security.yml/badge.svg)](https://github.com/cntm-labs/sentinel/actions/workflows/security.yml)
[![MSRV](https://img.shields.io/badge/rustc-1.85+-dea584?logo=rust&logoColor=white)](https://github.com/cntm-labs/sentinel/actions/workflows/ci.yml)

[![crates.io sntl](https://img.shields.io/crates/v/sntl?label=sntl&color=fc8d62)](https://crates.io/crates/sntl)
[![crates.io sntl-core](https://img.shields.io/crates/v/sntl-core?label=sntl-core&color=fc8d62)](https://crates.io/crates/sntl-core)
[![crates.io sntl-macros](https://img.shields.io/crates/v/sntl-macros?label=sntl-macros&color=fc8d62)](https://crates.io/crates/sntl-macros)
[![crates.io sntl-migrate](https://img.shields.io/crates/v/sntl-migrate?label=sntl-migrate&color=fc8d62)](https://crates.io/crates/sntl-migrate)
[![crates.io sntl-cli](https://img.shields.io/crates/v/sntl-cli?label=sntl-cli&color=fc8d62)](https://crates.io/crates/sntl-cli)
[![docs.rs](https://img.shields.io/docsrs/sntl?label=docs.rs)](https://docs.rs/sntl)

[![Rust](https://img.shields.io/badge/Rust-1.8k_LOC-dea584?logo=rust&logoColor=white)](sntl/)
[![Tests](https://img.shields.io/badge/Tests-1.8k_LOC-89e051)](sntl/tests/)
[![Config](https://img.shields.io/badge/Config-1k_LOC-89e051)](./)
[![Total Lines](https://img.shields.io/badge/Total-4.5k+_LOC-blue)](./)

[![Rust](https://img.shields.io/badge/Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-dea584?logo=rust&logoColor=white)](https://tokio.rs/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-4169E1?logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![rustls](https://img.shields.io/badge/rustls-dea584?logo=rust&logoColor=white)](https://github.com/rustls/rustls)

</div>

---

N+1 queries, over-fetching, unsafe relation access — caught at **compile time**, not production.

## Quick Start

```toml
[dependencies]
sntl = "0.1"
```

```rust
use sntl::prelude::*;

#[derive(Model)]
#[model(table = "users")]
struct User {
    #[primary_key]
    id: i64,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> Result<(), sntl::core::Error> {
    let config = Config::parse("postgres://user:pass@localhost/mydb")?;
    let pool = Pool::connect(config, 10).await?;
    let conn = pool.get().await?;

    // Type-safe query — wrong column names won't compile
    let users = User::select()
        .filter(User::EMAIL.eq("alice@example.com"))
        .fetch_all(&conn)
        .await?;

    Ok(())
}
```

## Compile-time SQL validation

Sentinel ships an sqlx-style `query!()` family that pulls types from a checked-in
`.sentinel/` cache. The schema and per-query metadata are produced by `sntl prepare`
against a live PostgreSQL, then committed alongside the code so CI builds work
offline.

```rust
use sntl::driver::Connection;

async fn examples(conn: &mut Connection) -> sntl::Result<()> {
    // Anonymous record — one struct field per output column.
    let row = sntl::query!("SELECT id, email FROM users WHERE id = $1", 42i32)
        .fetch_one(conn)
        .await?;
    let _: i32 = row.id;

    // Typed dispatch — your struct must impl FromRow.
    #[derive(sntl::FromRow)]
    struct User { id: i32, email: String }
    let user = sntl::query_as!(User, "SELECT id, email FROM users WHERE id = $1", 42i32)
        .fetch_one(conn)
        .await?;

    // Single-column projection.
    let count: i64 = sntl::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(conn)
        .await?;

    // Pipelined batch — single network round-trip for N queries.
    let _results = sntl::query_pipeline!(
        conn,
        a: "SELECT id FROM users WHERE id = $1", 1i32;
        b: "SELECT id FROM users WHERE id = $1", 2i32;
    ).await?;
    Ok(())
}
```

Bypass the cache temporarily with `sntl::query_unchecked!` / `query_as_unchecked!`,
or load SQL from disk with `sntl::query_file!` / `query_file_as!`.

The companion CLI provides:

```sh
sntl prepare   # scan workspace, pull schema, write .sentinel/
sntl check     # validate cache vs current source (CI-friendly)
sntl doctor    # diagnose config, DB, and cache health
```

Compared to sqlx: the offline cache is the source of truth (no DATABASE_URL required
at compile time); pipelined batches are first-class; nullable inference can be
overridden per-call with `nullable = [...]` / `non_null = [...]`.

See `docs/migration-from-sqlx.md` for a side-by-side migration guide.

## Features

- **Compile-time guards** — N+1, over-fetching, and unsafe relation access caught before runtime
- **Type-state relations** — `User<Bare>` vs `User<WithPosts>`, compile error on unloaded access
- **Partial types** — `#[derive(Partial)]` generates narrow select types, no over-fetching
- **Reducer pattern** — `#[reducer]` for transactions with auto-commit/rollback
- **Deadlock prevention** — auto-reorder locks by ID
- **4-layer query system** — from simple CRUD to raw SQL, always type-safe, always parameterized
- **Zero unsafe** in core — security by construction
- **Built on sentinel-driver** — SCRAM-SHA-256, pipeline mode, binary format, rustls

## Architecture

```
sentinel/
├── sntl           # Main crate — models, queries, transactions, types, query! family
├── sntl-macros    # Proc macros — derive(Model), derive(Partial), derive(FromRow), query!()
├── sntl-schema    # Shared SQL parsing, nullability, and .sentinel/ cache I/O
├── sntl-cli       # CLI binary — `sntl prepare`, `sntl check`, `sntl doctor`
├── sntl-core      # Core traits extraction (planned)
└── sntl-migrate   # Schema diff & migration generation (planned)
```

> `sntl`, `sntl-macros`, `sntl-schema`, and `sntl-cli` are implemented today.
> `sntl-core` and `sntl-migrate` are published on crates.io as name reservations
> and will be filled in in future releases.

## Development

```sh
cargo check --workspace                                # Type check
cargo test --workspace                                 # Run all tests
cargo clippy --workspace --all-targets -- -D warnings  # Lint
cargo fmt --all                                        # Format
```

## MSRV

Rust 1.85 (declared via `rust-version` in Cargo.toml).

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
