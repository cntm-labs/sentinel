# TechEmpower-spec benchmark: Sentinel vs a baseline driver

Two minimal `axum` apps that implement **all six** TechEmpower test types
over the same `world(id, randomNumber)` + `fortune(id, message)` schema.

## Layout

```
examples/
├── axum-bench/        # sntl + sentinel-driver (port 3000)
├── axum-sqlx-bench/   # baseline (port 3001)
└── README.md          # this file
```

Both apps:

- Use `axum 0.7` with no extra middleware
- Pool size 16
- Implement six TechEmpower endpoints: `/json`, `/plaintext`, `/db`,
  `/queries?queries=N`, `/updates?queries=N`, `/fortunes`
- Use the TechEmpower convention for multi-query endpoints: one
  connection per request, sequential queries on it
- Schema mirrors `toolset/databases/postgres/create-postgres.sql` from
  the archived TechEmpower repo, including the Japanese UTF-8 fortune
  row

## Quick run

```bash
# 1. Start a benchmark Postgres (max_connections lifted so 2 servers ×
#    16 connections each don't hit the default 100 cap).
podman run -d --name tfb-bench-pg --rm \
    -e POSTGRES_USER=bench -e POSTGRES_PASSWORD=bench -e POSTGRES_DB=bench \
    -p 5436:5432 postgres:16-alpine -c max_connections=300

# 2. Seed the world + fortune tables
podman exec -i tfb-bench-pg psql -U bench -d bench < axum-bench/sql/setup.sql

# 3. Build both (the baseline app needs a live DB at compile time
#    because its query macros are compile-time validated against the
#    schema in the live database — sntl validates against the
#    committed .sentinel/ cache instead).
(cd axum-bench && cargo build --release)
(cd axum-sqlx-bench && \
    DATABASE_URL=postgres://bench:bench@localhost:5436/bench \
    cargo build --release)

# 4. Start both servers in two terminals
DATABASE_URL=postgres://bench:bench@localhost:5436/bench \
    ./axum-bench/target/release/axum-bench         # listens on :3000
DATABASE_URL=postgres://bench:bench@localhost:5436/bench \
    ./axum-sqlx-bench/target/release/axum-sqlx-bench   # listens on :3001

# 5. Smoke-test
curl -s http://localhost:3000/db
curl -s http://localhost:3001/db

# 6. Drive load with oha (cargo install oha --locked)
oha -c 256 -z 5s --no-tui http://localhost:3000/db
oha -c 256 -z 5s --no-tui http://localhost:3001/db

# 7. Cleanup
pkill -f release/axum && podman rm -f tfb-bench-pg
```

## Verified numbers (2026-05-06, single-machine median of 3 runs)

Hardware: Linux 6.19.14-zen1-1-zen `x86_64`. Methodology: `oha -c 256 -z 5s`,
3 runs per cell, **median** reported (not best, not first). Fresh PG +
fresh server processes between phases; PG `max_connections=300` so
neither server can starve the other on the connection limit.

Same 16-connection pool both sides. Numbers in **req/s, higher is better**.

| Endpoint | sntl | Baseline | sntl vs Baseline |
|---|---:|---:|---:|
| `/json` | 1,344,303 | 1,383,474 | -3 % (axum/hyper-bound, no DB) |
| `/plaintext` | 1,368,388 | 1,367,285 | tied |
| `/db` | 148,309 | 75,025 | **+98 %** |
| `/fortunes` | 108,810 | 73,221 | **+49 %** |
| `/queries?queries=1` | 125,311 | 75,014 | **+67 %** |
| `/queries?queries=5` | 48,606 | 30,017 | **+62 %** |
| `/queries?queries=10` | 32,896 | 17,096 | **+92 %** |
| `/queries?queries=20` | 17,532 | 9,952 | **+76 %** |
| `/updates?queries=5` | 8,037 | 336 | **+2,295 %** |
| `/updates?queries=20` | 8,357 | 122 | **+6,778 %** |

### Reading the numbers

- **/json, /plaintext** — no database. Both apps tied at ≈1.37 M req/s
  because axum + hyper is the bottleneck. Sentinel doesn't try to win
  here; the equality is the relevant fact.
- **/db, /queries, /fortunes** — single-statement reads where Sentinel
  wins consistently. The macro emits a `query_typed_*` call per request
  that skips the standalone Parse round-trip; under load the savings
  compound across both pool turnover and TCP round-trips.
- **/updates** — the baseline collapses to ≈100–340 req/s under c=256.
  sentinel-driver's pool sustains 8 k req/s on the same workload. The
  most likely cause is the baseline's pool acquire serialising harder
  under heavy write load + Postgres row-level locks; sentinel-driver's
  pool absorbs the contention more gracefully. The behaviour reproduces
  across fresh runs and is documented as a follow-up driver
  investigation in the roadmap.

### What this is and isn't

This benchmark is **TechEmpower-spec compliant** in the source/schema
sense — same six endpoints, same `world` and `fortune` schema, same
load shape. It is **not** an official TechEmpower run because the
upstream `TechEmpower/FrameworkBenchmarks` repo was archived
2026-03-24 (read-only). The code under `examples/` is structured so it
can be lifted directly into `frameworks/Rust/sntl/` if a TFB successor
emerges.

Differences from a real TFB run:

- Workload generator is `oha` (Rust, native HTTP/1.1) rather than
  `wrk` with the official Lua scripts. Equivalent for non-pipelined
  endpoints; `/plaintext` numbers under TFB use HTTP/1.1 pipelining
  (16 reqs per TCP round-trip) which oha doesn't issue.
- Single-machine — load generator and PG share CPU with the servers.
  TFB runs them on separate hardware. Absolute numbers will shift on
  a separate-machine setup; relative ranking should hold.
- 5-second runs × 3 medians, not 15-second steady state.

For relative comparison between Sentinel and the baseline on the same
machine with controlled methodology, this is defensible.

## Prior numbers (PR #14, c=64, single shot)

The earlier benchmark in PR #14 used `oha -c 64 -n N` (single shot, no
median). The headline numbers there were directionally correct on
`/db` and `/updates` but overstated `/queries` scaling — single-shot
runs at low concurrency varied considerably. The numbers above
supersede them.
