use sentinel_core::expr::Column;
use sentinel_core::query::UpdateQuery;
use sentinel_core::types::Value;

#[test]
fn update_single_field() {
    let q = UpdateQuery::new("users")
        .set("name", "Alice Smith")
        .where_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "UPDATE \"users\" SET \"name\" = $1 WHERE \"id\" = $2 RETURNING *"
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0], Value::Text("Alice Smith".into()));
}

#[test]
fn update_multiple_fields() {
    let q = UpdateQuery::new("users")
        .set("name", "Alice Smith")
        .set("active", false)
        .where_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "UPDATE \"users\" SET \"name\" = $1, \"active\" = $2 WHERE \"id\" = $3 RETURNING *"
    );
    assert_eq!(binds.len(), 3);
}

#[test]
fn update_with_where_expr() {
    let col = Column::new("users", "role");
    let q = UpdateQuery::new("users")
        .set("active", false)
        .where_(col.eq("banned"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "UPDATE \"users\" SET \"active\" = $1 WHERE \"users\".\"role\" = $2 RETURNING *"
    );
    assert_eq!(binds.len(), 2);
}

#[test]
fn update_no_returning() {
    let q = UpdateQuery::new("users")
        .set("name", "Alice")
        .where_id(Value::from("id-1"))
        .no_returning();
    let (sql, _) = q.build();
    assert!(sql.ends_with("WHERE \"id\" = $2"));
}
