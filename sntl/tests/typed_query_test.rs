use sntl::core::expr::Column;
use sntl::core::query::{ModelQuery, TypedQuery};

#[test]
fn typed_query_builds_same_sql() {
    let q: ModelQuery = ModelQuery::from_table("users");
    let typed_q = q.Where(Column::new("users", "id").eq(1)).Typed();
    let (sql, binds) = typed_q.Build();
    assert!(sql.contains("users"));
    assert!(sql.contains("WHERE"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn typed_query_from_select() {
    let select = sntl::core::query::SelectQuery::new("posts")
        .where_(Column::new("posts", "published").eq(true));
    let typed = TypedQuery::from_select(select);
    let (sql, binds) = typed.Build();
    assert!(sql.contains("posts"));
    assert!(sql.contains("WHERE"));
    assert_eq!(binds.len(), 1);
}
