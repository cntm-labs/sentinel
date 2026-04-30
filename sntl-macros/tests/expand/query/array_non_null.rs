// Pass-case: non_null_elements override emits Vec<T>.

#[cfg(feature = "trybuild_fixtures")]
fn main() {
    async fn demo(conn: &mut sntl::driver::Connection) -> sntl::Result<()> {
        let row = sntl::query!(
            "SELECT tags FROM users",
            non_null_elements = [tags]
        )
        .fetch_one(conn)
        .await?;
        let _: Vec<String> = row.tags;
        Ok(())
    }
    let _ = demo;
}

#[cfg(not(feature = "trybuild_fixtures"))]
fn main() {}
