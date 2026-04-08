use sntl::core::Column;
use sntl::core::query::ModelQuery;

#[test]
fn pascal_find_builds_select() {
    let q = ModelQuery::from_table("users");
    let (sql, _) = q.Build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
}

#[test]
fn pascal_where_and_order() {
    let col = Column::new("users", "email");
    let q = ModelQuery::from_table("users")
        .Where(col.eq("test@test.com"))
        .OrderBy(Column::new("users", "name").asc())
        .Limit(10);
    let (sql, binds) = q.Build();
    assert!(sql.contains("WHERE"));
    assert!(sql.contains("ORDER BY"));
    assert!(sql.contains("LIMIT 10"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn pascal_offset() {
    let q = ModelQuery::from_table("users").Limit(10).Offset(20);
    let (sql, _) = q.Build();
    assert!(sql.contains("LIMIT 10"));
    assert!(sql.contains("OFFSET 20"));
}
