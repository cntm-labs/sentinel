# Contributing to Sentinel

## Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Run checks:
   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```
5. Commit and open a pull request

## Conventions

See [CLAUDE.md](CLAUDE.md) for project conventions, lint policy, and architecture.

Key rules:
- Zero `unsafe` in sntl-core
- All queries parameterized at every layer (no SQL injection possible)
- Every model field should have `doc = "..."` attribute
- Migrations are plain SQL files
- 100% test coverage target for sntl-core

## Pre-commit Hook

Pre-commit hooks are managed by `cargo-husky` and install automatically on first `cargo test`.

The hook runs:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace --quiet`

## Workspace Structure

```
sentinel/
├── sntl-core/       # Model trait, QueryBuilder, Transaction, Relations
├── sntl-macros/     # derive(Model), derive(Partial), #[reducer]
├── sntl-migrate/    # Schema diff, migration generation
├── sntl-cli/        # CLI binary (`sentinel` command)
├── examples/        # Usage examples
└── docs/            # Design and implementation plans
```
