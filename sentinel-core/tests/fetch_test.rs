use sentinel_core::query::SelectQuery;

// Test that build still works after adding fetch methods
#[test]
fn select_query_build_still_works() {
    let q = SelectQuery::new("users");
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
    assert!(binds.is_empty());
}

// Compile-time test: verify the methods exist with correct signatures
#[allow(dead_code)]
async fn fetch_api_compiles(conn: &mut sentinel_driver::Connection) {
    let q = SelectQuery::new("users");
    let _rows: Vec<sentinel_driver::Row> = q.fetch_all(conn).await.unwrap();

    let q2 = SelectQuery::new("users");
    let _row: sentinel_driver::Row = q2.fetch_one(conn).await.unwrap();

    let q3 = SelectQuery::new("users");
    let _row: Option<sentinel_driver::Row> = q3.fetch_optional(conn).await.unwrap();
}
