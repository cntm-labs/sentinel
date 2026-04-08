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
├── sntl/                # MAIN CRATE — all real code lives here
│   ├── src/
│   │   ├── core/        #   models, queries, transactions, types, expressions
│   │   ├── migrate.rs   #   migration module (in-crate, not extracted yet)
│   │   └── lib.rs       #   re-exports core + macros + driver
│   └── tests/
│       ├── pg_helpers/   #   shared helpers for integration tests (require_pg!, truncate)
│       ├── pg_exec_test.rs        # Integration: CRUD through query builders
│       ├── pg_transaction_test.rs # Integration: Transaction guard commit/rollback
│       ├── pg_value_roundtrip_test.rs # Integration: Value encode→PG→decode roundtrip
│       └── *.rs          #   unit tests (SQL generation, derive macros, etc.)
├── sntl-macros/         # REAL CODE — proc macros: derive(Model), derive(Partial)
│   └── src/
├── sntl-core/           # PLACEHOLDER — name reserved on crates.io, no real code yet
├── sntl-migrate/        # PLACEHOLDER — name reserved on crates.io, no real code yet
├── sntl-cli/            # PLACEHOLDER — name reserved on crates.io, prints stub message
├── tests/
│   └── integration/
│       └── setup.sql    # PostgreSQL schema for integration tests
├── docker-compose.yml   # PG 16 for local integration testing
├── docs/
│   └── plans/           # Design and implementation plans
└── Cargo.toml           # Workspace root
```

> **IMPORTANT:** Only `sntl` and `sntl-macros` contain real implementation.
> `sntl-core`, `sntl-migrate`, `sntl-cli` are published as name reservations only.
> Do NOT describe them as implemented in README or docs — mark them as `(planned)`.

## README & Docs Rules
- README.md is shared across all crates via `readme = "../README.md"` in each Cargo.toml
- Architecture section in README MUST reflect actual codebase state, not aspirational design
- Placeholder crates MUST be labeled `(planned)` in any public-facing docs
- When adding a new crate, update: workspace Cargo.toml, release-please config + manifest, both publish workflows, README badges + architecture, and this CLAUDE.md

## Build Commands
```sh
cargo check --workspace          # Type check
cargo test --workspace           # Run tests (integration tests skip without DATABASE_URL)
cargo clippy --workspace --all-targets -- -D warnings  # Lint
cargo fmt --all                  # Format
```

## Integration Tests (require live PostgreSQL)
```sh
docker compose up -d
psql postgres://sentinel:sentinel_test@localhost:5432/sentinel_test -f tests/integration/setup.sql
DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test cargo test
```
Integration tests (pg_*.rs) skip silently when DATABASE_URL is absent. CI runs them via `.github/workflows/postgresql.yml`.

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
- **NEVER use `#[allow(dead_code)]` or `#[allow(unused)]`** — delete unused code instead
- Every line of code must be used; no dead weight

## Related Projects
- **sentinel-driver** — PG wire protocol driver (separate repo at ../sentinel-driver)
- **layer-2** — Future realtime platform (at ../layer-2)

## Design Document
See `docs/plans/2026-04-03-sentinel-design.md` for full design.
