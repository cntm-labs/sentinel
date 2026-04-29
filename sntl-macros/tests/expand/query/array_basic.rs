// Pass-case: array column emits Vec<Option<T>> by default.

#[cfg(feature = "trybuild_fixtures")]
fn main() {
    async fn demo(conn: &mut sntl::driver::Connection) -> sntl::Result<()> {
        let row = sntl::query!("SELECT tags FROM users").fetch_one(conn).await?;
        let _: Vec<Option<String>> = row.tags;
        Ok(())
    }
    let _ = demo;
}

#[cfg(not(feature = "trybuild_fixtures"))]
fn main() {}
