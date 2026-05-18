//! End-to-end #[sntl::test] integration. Skips without SNTL_TEST_DATABASE_URL.

#[sntl::test]
async fn empty_db(pool: sntl::driver::Pool) -> anyhow::Result<()> {
    let n: i64 = sntl::query_scalar!("SELECT 42::int8")
        .fetch_one(&pool)
        .await?;
    assert_eq!(n, 42);
    Ok(())
}

#[sntl::test(migrations = "./tests/test_migrations")]
async fn with_schema(pool: sntl::driver::Pool) -> anyhow::Result<()> {
    let affected = sntl::query_unchecked!("INSERT INTO users (id, name) VALUES (1, 'alice')")
        .execute(&pool)
        .await?;
    assert_eq!(affected, 1);

    let (n,): (i64,) = sntl::query_unchecked!("SELECT COUNT(*)::int8 FROM users")
        .fetch_one::<(i64,)>(&pool)
        .await?;
    assert_eq!(n, 1);
    Ok(())
}

#[sntl::test(migrations = "./tests/test_migrations", fixtures("users", "posts"))]
async fn with_fixtures(pool: sntl::driver::Pool) -> anyhow::Result<()> {
    let (n,): (i64,) = sntl::query_unchecked!("SELECT COUNT(*)::int8 FROM posts WHERE user_id = 1")
        .fetch_one::<(i64,)>(&pool)
        .await?;
    assert_eq!(n, 2);
    Ok(())
}
