use sntl::core::expr::Column;
use sntl::core::query::{CursorQuery, ModelQuery};

#[test]
fn cursor_query_from_table_builds_sql() {
    let q = CursorQuery::from_table("users");
    let (sql, binds) = q.Build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
    assert!(binds.is_empty());
}

#[test]
fn cursor_query_with_where_and_order() {
    let q = CursorQuery::from_table("users")
        .Where(Column::new("users", "active").eq(true))
        .OrderBy(Column::new("users", "id").asc());
    let (sql, binds) = q.Build();
    assert!(sql.contains("WHERE"));
    assert!(sql.contains("ORDER BY"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn model_query_cursor_transition() {
    let q: ModelQuery = ModelQuery::from_table("users");
    let cursor = q.Where(Column::new("users", "id").gt(0)).Cursor();
    let (sql, _) = cursor.Build();
    assert!(sql.contains("users"));
    assert!(sql.contains("WHERE"));
}
