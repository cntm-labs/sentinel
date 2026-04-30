// Pass-case: query_as! with tuple target compiles and resolves to FromRow blanket impl.

#[cfg(feature = "trybuild_fixtures")]
fn main() {
    async fn demo(conn: &mut sntl::driver::Connection) -> sntl::Result<()> {
        let id: i32 = 1;
        let _: (i32,) = sntl::query_as!(
            (i32,),
            "SELECT id FROM users WHERE id = $1",
            id
        )
        .fetch_one(conn)
        .await?;
        Ok(())
    }
    let _ = demo;
}

#[cfg(not(feature = "trybuild_fixtures"))]
fn main() {}
