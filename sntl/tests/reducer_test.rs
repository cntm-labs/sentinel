//! Live-PG integration tests for `#[sntl::reducer]`.
//! Skips silently without DATABASE_URL.

use std::sync::{Arc, Mutex};

use sntl::driver::{Config, Connection, Event, Instrumentation};

#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        match ev {
            Event::ReducerBegin { name } => {
                self.0.lock().unwrap().push(format!("begin:{name}"));
            }
            Event::ReducerCommit { name, .. } => {
                self.0.lock().unwrap().push(format!("commit:{name}"));
            }
            Event::ReducerRollback { name, error } => {
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("rollback:{name}:{error}"));
            }
            _ => {}
        }
    }
}

async fn fresh_conn(rec: Arc<Recorder>) -> Option<Connection> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = Config::parse(&url)
        .ok()?
        .with_instrumentation(rec as Arc<dyn Instrumentation>);
    Connection::connect(cfg).await.ok()
}

async fn truncate(conn: &mut Connection) {
    conn.execute("DELETE FROM \"posts\"", &[]).await.ok();
    conn.execute("DELETE FROM \"users\"", &[]).await.ok();
}

async fn count_users(conn: &mut Connection) -> i64 {
    let row = conn
        .query_one("SELECT COUNT(*)::int8 FROM users", &[])
        .await
        .unwrap();
    row.try_get::<i64>(0).unwrap()
}

// ────────── Scenario 1: success commits ──────────

#[sntl::reducer]
async fn insert_one(conn: &mut Connection, name: &str) -> sntl::Result<()> {
    let email = format!("{name}@example.com");
    conn.execute(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        &[&name as &(dyn sntl::driver::ToSql + Sync), &email],
    )
    .await
    .map_err(sntl::Error::from)?;
    Ok(())
}

#[tokio::test]
async fn success_commits() {
    let rec = Arc::new(Recorder::default());
    let Some(mut conn) = fresh_conn(rec.clone()).await else {
        return;
    };
    truncate(&mut conn).await;

    insert_one(&mut conn, "alice").await.unwrap();

    assert_eq!(count_users(&mut conn).await, 1);

    let events = rec.0.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], "begin:insert_one");
    assert!(events[1].starts_with("commit:insert_one"));
}

// ────────── Scenario 2: error rollbacks ──────────

#[sntl::reducer]
async fn insert_then_fail(conn: &mut Connection, name: &str) -> sntl::Result<()> {
    let email = format!("{name}@example.com");
    conn.execute(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        &[&name as &(dyn sntl::driver::ToSql + Sync), &email],
    )
    .await
    .map_err(sntl::Error::from)?;
    Err(sntl::Error::Transaction("forced failure".into()))
}

#[tokio::test]
async fn error_rollbacks() {
    let rec = Arc::new(Recorder::default());
    let Some(mut conn) = fresh_conn(rec.clone()).await else {
        return;
    };
    truncate(&mut conn).await;

    let res = insert_then_fail(&mut conn, "bob").await;
    assert!(res.is_err());

    assert_eq!(
        count_users(&mut conn).await,
        0,
        "rollback should erase the insert"
    );

    let events = rec.0.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], "begin:insert_then_fail");
    // Error::Transaction displays as "transaction error: forced failure"
    assert!(events[1].starts_with("rollback:insert_then_fail:"));
    assert!(events[1].contains("forced failure"));
}

// ────────── Scenario 3: panic rollbacks + resumes ──────────

#[sntl::reducer]
async fn insert_then_panic(conn: &mut Connection, name: &str) -> sntl::Result<()> {
    let email = format!("{name}@example.com");
    conn.execute(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        &[&name as &(dyn sntl::driver::ToSql + Sync), &email],
    )
    .await
    .map_err(sntl::Error::from)?;
    panic!("synthetic panic");
}

#[tokio::test]
async fn panic_rollbacks_and_resumes() {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;

    let rec = Arc::new(Recorder::default());
    let Some(mut conn) = fresh_conn(rec.clone()).await else {
        return;
    };
    truncate(&mut conn).await;

    let panicked = AssertUnwindSafe(insert_then_panic(&mut conn, "carol"))
        .catch_unwind()
        .await
        .is_err();
    assert!(panicked, "panic should propagate past the reducer wrapper");

    assert_eq!(
        count_users(&mut conn).await,
        0,
        "rollback should erase the insert even on panic"
    );

    let events = rec.0.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], "begin:insert_then_panic");
    assert_eq!(events[1], "rollback:insert_then_panic:panic");
}

// ────────── Scenario 4: serializable isolation ──────────

#[sntl::reducer(isolation = "serializable")]
async fn cas_inc(conn: &mut Connection, id: i32) -> sntl::Result<i64> {
    let row = conn
        .query_one(
            "SELECT counter::int8 FROM kv WHERE id = $1",
            &[&id as &(dyn sntl::driver::ToSql + Sync)],
        )
        .await
        .map_err(sntl::Error::from)?;
    let current: i64 = row.try_get(0).map_err(sntl::Error::from)?;
    let new_val = current + 1;
    conn.execute(
        "UPDATE kv SET counter = $1 WHERE id = $2",
        &[&new_val as &(dyn sntl::driver::ToSql + Sync), &id],
    )
    .await
    .map_err(sntl::Error::from)?;
    Ok(new_val)
}

#[tokio::test]
async fn serializable_isolation_applied() {
    let rec = Arc::new(Recorder::default());
    let Some(mut conn1) = fresh_conn(rec.clone()).await else {
        return;
    };
    let Some(mut conn2) = fresh_conn(rec.clone()).await else {
        return;
    };

    // Seed kv table fresh for this test.
    // Suppress NOTICE from "IF EXISTS" drop so the driver protocol isn't confused.
    conn1
        .execute("SET client_min_messages = ERROR", &[])
        .await
        .unwrap();
    conn1.execute("DROP TABLE IF EXISTS kv", &[]).await.unwrap();
    conn1
        .execute("RESET client_min_messages", &[])
        .await
        .unwrap();
    conn1
        .execute("CREATE TABLE kv (id int PRIMARY KEY, counter int)", &[])
        .await
        .unwrap();
    conn1
        .execute("INSERT INTO kv (id, counter) VALUES (1, 0)", &[])
        .await
        .unwrap();

    let (r1, r2) = tokio::join!(cas_inc(&mut conn1, 1), cas_inc(&mut conn2, 1));

    let ok_count = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    let err_count = 2 - ok_count;
    assert!(
        ok_count >= 1 && err_count <= 1,
        "expected at most one reducer to fail with serialization_failure (got r1={:?}, r2={:?})",
        r1,
        r2
    );

    // Clean up kv table so it doesn't leak into other test runs.
    // Suppress NOTICE in case table is already gone.
    conn1
        .execute("SET client_min_messages = ERROR", &[])
        .await
        .ok();
    conn1.execute("DROP TABLE IF EXISTS kv", &[]).await.ok();
}
