//! Shared helpers for PostgreSQL integration tests.
//!
//! All pg_* tests skip silently when DATABASE_URL is not set.

/// Return DATABASE_URL or skip the test.
macro_rules! require_pg {
    () => {
        match std::env::var("DATABASE_URL").ok() {
            Some(url) => url,
            None => return,
        }
    };
}

/// Clean a table before a test (DELETE all rows).
pub async fn truncate(conn: &mut sntl::driver::Connection, table: &str) {
    conn.execute(&format!("TRUNCATE \"{table}\" CASCADE"), &[])
        .await
        .unwrap();
}
