# TechEmpower-style microbench: Sentinel vs sqlx

Two minimal `axum` apps that implement the TechEmpower **/db**, **/queries**,
and **/updates** endpoints over the same `world(id, randomNumber)` schema.

## Layout

```
examples/
├── axum-bench/        # sntl + sentinel-driver (port 3000)
├── axum-sqlx-bench/   # sqlx (port 3001)
└── README.md          # this file
```

Both apps:
- Use `axum 0.7` with no extra middleware
- Pool size 16
- Use the **TechEmpower pattern**: one connection per request, sequential
  queries on it (avoids pool-acquire contention that the spawn-per-query
  pattern triggers)

## Quick run

```bash
# 1. Start a benchmark Postgres
podman run -d --name bench-pg --rm \
    -e POSTGRES_USER=bench -e POSTGRES_PASSWORD=bench -e POSTGRES_DB=bench \
    -p 5434:5432 postgres:16-alpine

# 2. Seed the world table (10000 rows)
podman exec -i bench-pg psql -U bench -d bench < axum-bench/sql/setup.sql

# 3. Build both
(cd axum-bench && cargo build --release)
(cd axum-sqlx-bench && \
    DATABASE_URL=postgres://bench:bench@localhost:5434/bench \
    cargo build --release)

# 4. Start both servers (separate terminals)
DATABASE_URL=postgres://bench:bench@localhost:5434/bench \
    ./axum-bench/target/release/axum-bench         # listens on :3000
DATABASE_URL=postgres://bench:bench@localhost:5434/bench \
    ./axum-sqlx-bench/target/release/axum-sqlx-bench   # listens on :3001

# 5. Benchmark with oha (cargo install oha)
oha -n 30000 -c 64 --no-tui http://localhost:3000/db
oha -n 5000 -c 64 --no-tui http://localhost:3000/queries?queries=20
oha -n 3000 -c 64 --no-tui http://localhost:3000/updates?queries=20

# Compare with sqlx baseline on :3001 the same way.

# 6. Tear down
podman rm -f bench-pg
```

## Headline results (local, AMD64, single PG container)

`oha -n N -c 64`. Higher = better. All endpoints use 1-conn-per-request,
sequential queries — the TechEmpower convention.

| Endpoint | Sentinel r/s | sqlx r/s | Sentinel vs sqlx |
|---|---:|---:|---:|
| `/db` | 117,561 | 88,489 | **+33 %** |
| `/queries?queries=1` | 93,954 | 76,666 | **+23 %** |
| `/queries?queries=5` | 35,655 | 13,808 | **+158 %** |
| `/queries?queries=10` | 18,611 | 7,733 | **+141 %** |
| `/queries?queries=20` | 9,872 | 3,489 | **+183 %** |
| `/updates?queries=5` | 1,145 | 293 | **+291 %** |
| `/updates?queries=20` | 291 | 72 | **+304 %** |

## Why Sentinel wins (and where it can lose)

**Wins:**
- `query_typed_*` skips the standalone `Parse` round-trip. The macro emits
  the parameter OIDs inline so each request is one Parse+Bind+Describe+Execute
  message instead of two round-trips.
- Driver's 2-tier statement cache (HashMap + LRU 256) keeps the Parse cost
  amortised on top of that.
- The advantage **compounds with N**: at `N=1` Sentinel is +23 %; at `N=20`
  it's +183 %. Each query in the request saves the Parse round-trip.
- For `/updates` the gap widens further (+304 % at N=20) because each
  request issues two statements — both benefit from the skip.

**Where the spawn-per-query anti-pattern bites:**
- An earlier draft of these handlers spawned one task per query and
  `pool.acquire()`-ed inside each spawn. That triggers heavy pool-acquire
  contention. Under that pattern Sentinel **lost** /queries to sqlx because
  the sentinel-driver pool's acquire fairness is less optimised than
  sqlx-postgres's. The current handlers use the TechEmpower-correct pattern;
  pool-acquire optimisation is on the v0.3 roadmap.

## Caveats

- One-machine benchmark with the load generator and PG sharing CPU.
  TechEmpower runs the load generator on a separate physical box; expect
  absolute numbers to shift, but the *relative* ranking should hold.
- Both apps use `axum::serve` defaults; no h2/h2c, no keep-alive tuning.
- The `world` schema is hand-seeded with `random()`; TechEmpower mandates
  10 000 rows but doesn't pin the values.
- Update path uses two statements (SELECT then UPDATE) instead of a single
  `UPDATE ... RETURNING`. TechEmpower allows either.
