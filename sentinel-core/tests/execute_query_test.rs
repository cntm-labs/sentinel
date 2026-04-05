use sentinel_core::query::{DeleteQuery, InsertQuery};

#[test]
fn insert_build_still_works() {
    let q = InsertQuery::new("users").column("email", "alice@example.com");
    let (sql, binds) = q.build();
    assert!(sql.contains("INSERT INTO"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn delete_build_still_works() {
    let q = DeleteQuery::new("users").where_id(sentinel_core::types::Value::Int(1));
    let (sql, binds) = q.build();
    assert!(sql.contains("DELETE FROM"));
    assert_eq!(binds.len(), 1);
}

// Compile-time test: verify execute methods exist
#[allow(dead_code)]
async fn execute_api_compiles(conn: &mut sentinel_driver::Connection) {
    let q = InsertQuery::new("users").column("email", "test@test.com");
    let _rows: Vec<sentinel_driver::Row> = q.fetch_returning(conn).await.unwrap();

    let q2 = DeleteQuery::new("users").where_id(sentinel_core::types::Value::Int(1));
    let _affected: u64 = q2.execute(conn).await.unwrap();
}
