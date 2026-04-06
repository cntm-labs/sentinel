<div align="center">

# sentinel

**Compile-time guarded ORM for PostgreSQL — your data's guardian from compile to production.**

[![CI](https://github.com/cntm-labs/sentinel/actions/workflows/ci.yml/badge.svg)](https://github.com/cntm-labs/sentinel/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/cntm-labs/sentinel/branch/main/graph/badge.svg)](https://codecov.io/gh/cntm-labs/sentinel)

[![crates.io sntl](https://img.shields.io/crates/v/sntl?label=sntl&color=fc8d62)](https://crates.io/crates/sntl)
[![crates.io sntl-core](https://img.shields.io/crates/v/sntl-core?label=sntl-core&color=fc8d62)](https://crates.io/crates/sntl-core)
[![crates.io sntl-macros](https://img.shields.io/crates/v/sntl-macros?label=sntl-macros&color=fc8d62)](https://crates.io/crates/sntl-macros)
[![docs.rs](https://img.shields.io/docsrs/sntl?label=docs.rs)](https://docs.rs/sntl)

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
crates/
├── sntl           # Umbrella crate — cargo add sntl, ready to go
├── sntl-core      # Model trait, QueryBuilder, types, connection
├── sntl-macros    # derive(Model), derive(Partial), #[reducer]
├── sntl-migrate   # Schema diff, migration generation
└── sntl-cli       # CLI binary (sentinel command)
```

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
