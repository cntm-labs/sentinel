# CLAUDE.md — Sentinel ORM

## Overview
Sentinel is a compile-time guarded Rust ORM for PostgreSQL. Standalone crate on crates.io.
Tagline: "Your data's guardian — from compile to production"

## Tech Stack
- **Language:** Rust (stable)
- **Database:** PostgreSQL (only, v1)
- **Async:** tokio
- **Driver:** sentinel-driver (custom PG wire protocol, separate repo)
- **TLS:** rustls (no OpenSSL)

## Workspace Structure
```
sentinel/
├── sntl-core/           # Model trait, QueryBuilder, Transaction, Relations
│   └── src/
├── sntl-macros/         # derive(Model), derive(Partial), #[reducer]
│   └── src/
├── sntl-migrate/        # Schema diff, migration generation
│   └── src/
├── sntl-cli/            # CLI binary (`sentinel` command)
│   └── src/
├── examples/            # Usage examples
├── docs/
│   └── plans/           # Design and implementation plans
└── Cargo.toml           # Workspace root
```

## Build Commands
```sh
cargo check --workspace          # Type check
cargo test --workspace           # Run tests
cargo clippy --workspace --all-targets -- -D warnings  # Lint
cargo fmt --all                  # Format
```

## Design Principles
1. **Guard at compile-time** — N+1, over-fetching, unsafe relation access caught at compile
2. **Zero surprise** — no lazy loading, no hidden queries, every DB call explicit
3. **No cliff** — 4-layer query system, always type-safe, always parameterized

## Key Patterns
- **Type-state pattern** for relations: `User<Bare>` vs `User<WithPosts>` — compile error if accessing unloaded relation
- **Partial types** for select: `#[derive(Partial)]` generates narrow return types
- **Reducer pattern** for transactions: `#[reducer]` = auto-commit/rollback
- **Deadlock prevention**: auto-reorder locks by ID

## Conventions
- Zero `unsafe` in sntl-core
- All queries parameterized at every layer (no SQL injection possible)
- Migrations are plain SQL files
- Every model field should have `doc = "..."` attribute
- 100% test coverage target

## Related Projects
- **sentinel-driver** — PG wire protocol driver (separate repo at ../sentinel-driver)
- **layer-2** — Future realtime platform (at ../layer-2)

## Design Document
See `docs/plans/2026-04-03-sentinel-design.md` for full design.
