//! TechEmpower-spec benchmark for `sqlx + axum` (comparison baseline).
//! Same six endpoints as `axum-bench`. Listens on port 3001.

use axum::{
    Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json},
    routing::get,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};

#[derive(Serialize)]
struct Message {
    message: &'static str,
}

#[derive(Serialize)]
struct World {
    id: i32,
    #[serde(rename = "randomNumber")]
    random_number: i32,
}

struct Fortune {
    id: i32,
    message: String,
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

async fn json() -> Json<Message> {
    Json(Message {
        message: "Hello, World!",
    })
}

async fn plaintext() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/plain")], "Hello, World!")
}

async fn db(State(pool): State<PgPool>) -> Result<Json<World>, StatusCode> {
    let id = rand_id();
    let row = sqlx::query!("SELECT id, randomnumber FROM world WHERE id = $1", id)
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
    let mut conn = pool
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = rand_id();
        let row = sqlx::query!("SELECT id, randomnumber FROM world WHERE id = $1", id)
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
    let mut conn = pool
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = rand_id();
        let row = sqlx::query!("SELECT id, randomnumber FROM world WHERE id = $1", id)
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

async fn fortunes(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = sqlx::query!("SELECT id, message FROM fortune")
        .fetch_all(&mut *conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut fortunes: Vec<Fortune> = rows
        .into_iter()
        .map(|r| Fortune {
            id: r.id,
            message: r.message,
        })
        .collect();
    fortunes.push(Fortune {
        id: 0,
        message: "Additional fortune added at request time.".to_string(),
    });
    fortunes.sort_by(|a, b| a.message.cmp(&b.message));

    let mut html = String::with_capacity(2048);
    html.push_str(
        "<!DOCTYPE html><html><head><title>Fortunes</title></head>\
        <body><table><tr><th>id</th><th>message</th></tr>",
    );
    for f in &fortunes {
        html.push_str("<tr><td>");
        html.push_str(&f.id.to_string());
        html.push_str("</td><td>");
        escape_html_into(&f.message, &mut html);
        html.push_str("</td></tr>");
    }
    html.push_str("</table></body></html>");
    Ok((
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    ))
}

fn escape_html_into(input: &str, out: &mut String) {
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
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
        .route("/json", get(json))
        .route("/plaintext", get(plaintext))
        .route("/db", get(db))
        .route("/queries", get(queries))
        .route("/updates", get(updates))
        .route("/fortunes", get(fortunes))
        .with_state(pool);

    let addr = "0.0.0.0:3001";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("axum-sqlx-bench listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
