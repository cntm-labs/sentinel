//! TechEmpower-style microbench for `sqlx + axum` (comparison baseline).
//!
//! Same endpoints + schema as `axum-bench`. Run on different port (3001).

use axum::{Router, extract::{Query, State}, http::StatusCode, response::Json, routing::get};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};

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

async fn db(State(pool): State<PgPool>) -> Result<Json<World>, StatusCode> {
    let id = rand_id();
    let row = sqlx::query!(
        "SELECT id, randomnumber FROM world WHERE id = $1",
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(World {
        id: row.id,
        random_number: row.randomnumber,
    }))
}

async fn queries(
    State(pool): State<PgPool>,
    Query(params): Query<QueriesParams>,
) -> Result<Json<Vec<World>>, StatusCode> {
    let n = clamp_n(params.queries);
    // TechEmpower pattern: one connection per request, sequential queries on it.
    let mut conn = pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = rand_id();
        let row = sqlx::query!(
            "SELECT id, randomnumber FROM world WHERE id = $1",
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        out.push(World {
            id: row.id,
            random_number: row.randomnumber,
        });
    }
    Ok(Json(out))
}

async fn updates(
    State(pool): State<PgPool>,
    Query(params): Query<QueriesParams>,
) -> Result<Json<Vec<World>>, StatusCode> {
    let n = clamp_n(params.queries);
    let mut conn = pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = rand_id();
        let row = sqlx::query!(
            "SELECT id, randomnumber FROM world WHERE id = $1",
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let new_rn = rand::thread_rng().gen_range(1..=10_000);
        sqlx::query!(
            "UPDATE world SET randomnumber = $2 WHERE id = $1",
            row.id,
            new_rn
        )
        .execute(&mut *conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        out.push(World {
            id: row.id,
            random_number: new_rn,
        });
    }
    Ok(Json(out))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://bench:bench@localhost:5432/bench".into());
    let pool = PgPoolOptions::new()
        .max_connections(16)
        .connect(&url)
        .await?;

    let app = Router::new()
        .route("/db", get(db))
        .route("/queries", get(queries))
        .route("/updates", get(updates))
        .with_state(pool);

    let addr = "0.0.0.0:3001";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("axum-sqlx-bench listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
