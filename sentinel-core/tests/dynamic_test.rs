use sentinel_core::query::QueryBuilder;
use sentinel_core::types::Value;

#[test]
fn dynamic_select() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.column("email");
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"id\", \"email\" FROM \"users\"");
    assert!(binds.is_empty());
}

#[test]
fn dynamic_select_with_where() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.where_eq("active", true);
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"id\" FROM \"users\" WHERE \"active\" = $1");
    assert_eq!(binds.len(), 1);
}

#[test]
fn dynamic_select_multiple_where() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.where_eq("active", true);
    q.where_eq("role", "admin");
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"id\" FROM \"users\" WHERE \"active\" = $1 AND \"role\" = $2"
    );
    assert_eq!(binds.len(), 2);
}

#[test]
fn dynamic_select_order_limit() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.order_by_desc("created_at");
    q.limit(10);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"id\" FROM \"users\" ORDER BY \"created_at\" DESC LIMIT 10"
    );
}

#[test]
fn always_parameterized() {
    let mut q = QueryBuilder::select_from("users");
    q.column("id");
    q.where_eq("name", "Robert'); DROP TABLE users;--");
    let (sql, binds) = q.build();
    // Value is in binds, NOT in SQL string
    assert_eq!(sql, "SELECT \"id\" FROM \"users\" WHERE \"name\" = $1");
    assert_eq!(
        binds[0],
        Value::Text("Robert'); DROP TABLE users;--".into())
    );
}
