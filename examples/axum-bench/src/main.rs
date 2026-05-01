//! TechEmpower-style microbench for `sntl + axum`.
//!
//! Endpoints:
//! - GET /db                — single random World row
//! - GET /queries?queries=N — N concurrent random rows
//! - GET /updates?queries=N — N concurrent fetch+update+return
//!
//! Run:
//!   docker run --rm -d --name bench-pg -p 5432:5432 \
//!     -e POSTGRES_USER=bench -e POSTGRES_PASSWORD=bench -e POSTGRES_DB=bench \
//!     postgres:16-alpine
//!   psql postgres://bench:bench@localhost:5432/bench -f sql/setup.sql
//!   DATABASE_URL=postgres://bench:bench@localhost:5432/bench cargo run --release

use axum::{Router, extract::{Query, State}, http::StatusCode, response::Json, routing::get};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sntl::driver::{Config, Pool};
use sntl::driver::pool::config::PoolConfig;
use std::sync::Arc;

#[derive(Serialize)]
struct World {
    id: i32,
    #[serde(rename = "randomNumber")]
    random_number: i32,
}

#[derive(Deserialize)]
struct QueriesParams {
    queries: Option<i32>,
}

fn clamp_n(n: Option<i32>) -> i32 {
    n.unwrap_or(1).clamp(1, 500)
}

fn rand_id() -> i32 {
    rand::thread_rng().gen_range(1..=10_000)
}

type AppState = Arc<Pool>;

async fn db(State(pool): State<AppState>) -> Result<Json<World>, StatusCode> {
    let mut conn = pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let id = rand_id();
    let row = sntl::query!("SELECT id, randomnumber FROM world WHERE id = $1", id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(World { id: row.id, random_number: row.randomnumber }))
}

async fn queries(
    State(pool): State<AppState>,
    Query(params): Query<QueriesParams>,
) -> Result<Json<Vec<World>>, StatusCode> {
    let n = clamp_n(params.queries);
    // TechEmpower pattern: one connection per request, sequential queries on it.
    // Avoids pool-acquire contention that the spawn-per-query pattern triggers.
    let mut conn = pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = rand_id();
        let row = sntl::query!("SELECT id, randomnumber FROM world WHERE id = $1", id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        out.push(World { id: row.id, random_number: row.randomnumber });
    }
    Ok(Json(out))
}

async fn updates(
    State(pool): State<AppState>,
    Query(params): Query<QueriesParams>,
) -> Result<Json<Vec<World>>, StatusCode> {
    let n = clamp_n(params.queries);
    let mut conn = pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = rand_id();
        let row = sntl::query!("SELECT id, randomnumber FROM world WHERE id = $1", id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let new_rn = rand::thread_rng().gen_range(1..=10_000);
        sntl::query!(
            "UPDATE world SET randomnumber = $2 WHERE id = $1",
            row.id,
            new_rn
        )
        .execute(&mut *conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        out.push(World { id: row.id, random_number: new_rn });
    }
    Ok(Json(out))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://bench:bench@localhost:5432/bench".into());
    let cfg = Config::parse(&url)?;
    let pool = Arc::new(Pool::new(cfg, PoolConfig::new().max_connections(16)));

    let app = Router::new()
        .route("/db", get(db))
        .route("/queries", get(queries))
        .route("/updates", get(updates))
        .with_state(pool);

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("axum-bench (sntl) listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
