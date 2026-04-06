use sntl::core::expr::Column;
use sntl::core::query::DeleteQuery;
use sntl::core::types::Value;

#[test]
fn delete_by_id() {
    let q = DeleteQuery::new("users").where_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1");
    assert_eq!(binds.len(), 1);
}

#[test]
fn delete_with_where_expr() {
    let col = Column::new("users", "active");
    let q = DeleteQuery::new("users").where_(col.eq(false));
    let (sql, binds) = q.build();
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"users\".\"active\" = $1");
    assert_eq!(binds.len(), 1);
}

#[test]
fn delete_with_returning() {
    let q = DeleteQuery::new("users")
        .where_id(Value::from("abc-123"))
        .returning();
    let (sql, _) = q.build();
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1 RETURNING *");
}
