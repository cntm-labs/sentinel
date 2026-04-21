// Pass-case smoke test for sntl::query!. Requires the committed .sentinel/
// cache at the workspace root; trybuild walks up CARGO_MANIFEST_DIR to find
// it the same way the proc-macro does.

#[cfg(feature = "trybuild_fixtures")]
fn main() {
    async fn demo(conn: &mut sntl::driver::Connection) -> sntl::Result<()> {
        let id = uuid::Uuid::new_v4();
        let _row = sntl::query!("SELECT id FROM users WHERE id = $1", id)
            .fetch_one(conn)
            .await?;
        Ok(())
    }
    let _ = demo;
}

#[cfg(not(feature = "trybuild_fixtures"))]
fn main() {}
