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

/// Clean all test tables before a test.
///
/// Deletes in dependency order (posts → users) to respect foreign keys.
/// Uses DELETE instead of TRUNCATE to avoid NoticeResponse from CASCADE
/// which sentinel-driver does not yet handle.
pub async fn clean_tables(conn: &mut sntl::driver::Connection) {
    conn.execute("DELETE FROM \"posts\"", &[]).await.unwrap();
    conn.execute("DELETE FROM \"users\"", &[]).await.unwrap();
    conn.execute("DELETE FROM \"type_roundtrip\"", &[])
        .await
        .unwrap();
}
