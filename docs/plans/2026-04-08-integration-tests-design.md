# Integration Test Infrastructure Design

**Date:** 2026-04-08
**Status:** Implementing
**Blocks:** Phase 3 completion (exec/transaction validation), all future phases

## Goal

Validate that ORM query builders, execution methods, and transaction guard work against a real PostgreSQL instance. Unit tests only verify SQL generation — integration tests prove correctness end-to-end.

## Components

### 1. `docker-compose.yml` (repo root)

Single PG 16 container for local development. Credentials match CI.

### 2. `tests/integration/setup.sql`

Test schema:
- `users` — standard model (id serial, name text, email text, active bool, created_at timestamptz)
- `posts` — relation target for Phase 4 (id serial, user_id int references users, title text, body text, published bool, created_at timestamptz)
- `type_roundtrip` — every Value variant (bool, int4, int8, float8, text, uuid, timestamptz, bytea)

### 3. `tests/integration/` module

- `mod.rs` — `require_pg!()` macro (skip if no DATABASE_URL), `connect()` helper
- `exec_test.rs` — CRUD through typed query builders (SelectQuery, InsertQuery, UpdateQuery, DeleteQuery)
- `transaction_test.rs` — commit persists, rollback reverts, drop-without-commit reverts
- `value_roundtrip_test.rs` — insert each Value variant, read back, assert equality

### 4. `.github/workflows/postgresql.yml`

PG 16 + 17 service containers. Runs setup.sql then `cargo test --workspace`.

## Skip mechanism

`require_pg!()` returns early (not panic) when DATABASE_URL is absent. Regular `cargo test` skips integration tests. CI sets DATABASE_URL to run them.

## Local dev workflow

```sh
docker compose up -d
psql postgres://sentinel:sentinel_test@localhost:5432/sentinel_test -f tests/integration/setup.sql
DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test cargo test
```
