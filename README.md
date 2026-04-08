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
├── sntl           # Main crate — models, queries, transactions, types (cargo add sntl)
├── sntl-macros    # Proc macros — derive(Model), derive(Partial)
├── sntl-core      # Core traits extraction (planned)
├── sntl-migrate   # Schema diff & migration generation (planned)
└── sntl-cli       # CLI binary — sentinel command (planned)
```

> `sntl` is the only crate with implementation today. The others are published on crates.io
> as name reservations and will be extracted/implemented in future releases.

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
