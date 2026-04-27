// Pass-case smoke test for sntl::query!. Requires the committed .sentinel/
// cache at the workspace root; trybuild walks up CARGO_MANIFEST_DIR to find
// it the same way the proc-macro does. The schema mirrors
// tests/integration/setup.sql so this stays in sync with the live-PG test.

#[cfg(feature = "trybuild_fixtures")]
fn main() {
    async fn demo(conn: &mut sntl::driver::Connection) -> sntl::Result<()> {
        let id: i32 = 1;
        let _row = sntl::query!("SELECT id FROM users WHERE id = $1", id)
            .fetch_one(conn)
            .await?;
        Ok(())
    }
    let _ = demo;
}

#[cfg(not(feature = "trybuild_fixtures"))]
fn main() {}
