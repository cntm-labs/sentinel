use sntl::core::expr::Column;
use sntl::core::types::Value;

#[test]
fn column_eq_generates_expr() {
    let col = Column::new("users", "email");
    let expr = col.eq("alice@example.com");
    assert_eq!(expr.to_sql(1), "\"users\".\"email\" = $1");
    assert_eq!(expr.binds(), vec![Value::Text("alice@example.com".into())]);
}

#[test]
fn column_ne_generates_expr() {
    let col = Column::new("users", "role");
    let expr = col.ne("admin");
    assert_eq!(expr.to_sql(1), "\"users\".\"role\" != $1");
}

#[test]
fn column_gt_lt() {
    let col = Column::new("users", "age");
    assert_eq!(col.gt(25i32).to_sql(1), "\"users\".\"age\" > $1");
    assert_eq!(col.lt(65i32).to_sql(1), "\"users\".\"age\" < $1");
    assert_eq!(col.gte(25i32).to_sql(1), "\"users\".\"age\" >= $1");
    assert_eq!(col.lte(65i32).to_sql(1), "\"users\".\"age\" <= $1");
}

#[test]
fn column_is_null() {
    let col = Column::new("users", "deleted_at");
    let expr = col.is_null();
    assert_eq!(expr.to_sql(1), "\"users\".\"deleted_at\" IS NULL");
    assert!(expr.binds().is_empty());
}

#[test]
fn column_is_not_null() {
    let col = Column::new("users", "email");
    let expr = col.is_not_null();
    assert_eq!(expr.to_sql(1), "\"users\".\"email\" IS NOT NULL");
}

#[test]
fn column_like() {
    let col = Column::new("users", "name");
    let expr = col.like("%alice%");
    assert_eq!(expr.to_sql(1), "\"users\".\"name\" LIKE $1");
    assert_eq!(expr.binds(), vec![Value::Text("%alice%".into())]);
}

#[test]
fn column_in_list() {
    let col = Column::new("users", "status");
    let expr = col.in_list(vec![Value::from("active"), Value::from("pending")]);
    assert_eq!(expr.to_sql(1), "\"users\".\"status\" IN ($1, $2)");
    assert_eq!(expr.binds().len(), 2);
}

#[test]
fn expr_and_combines() {
    let c1 = Column::new("users", "age");
    let c2 = Column::new("users", "active");
    let expr = c1.gt(18i32).and(c2.eq(true));
    assert_eq!(
        expr.to_sql(1),
        "(\"users\".\"age\" > $1 AND \"users\".\"active\" = $2)"
    );
}

#[test]
fn expr_or_combines() {
    let c1 = Column::new("users", "role");
    let c2 = Column::new("users", "role");
    let expr = c1.eq("admin").or(c2.eq("moderator"));
    assert_eq!(
        expr.to_sql(1),
        "(\"users\".\"role\" = $1 OR \"users\".\"role\" = $2)"
    );
}

#[test]
fn column_desc_asc() {
    let col = Column::new("users", "created_at");
    assert_eq!(col.desc().to_sql_bare(), "\"users\".\"created_at\" DESC");
    assert_eq!(col.asc().to_sql_bare(), "\"users\".\"created_at\" ASC");
}

#[test]
fn bind_index_chains_correctly() {
    let c1 = Column::new("users", "name");
    let c2 = Column::new("users", "email");
    let c3 = Column::new("users", "age");
    let expr = c1.eq("Alice").and(c2.eq("alice@ex.com")).and(c3.gt(18i32));
    let sql = expr.to_sql(1);
    // Should produce $1, $2, $3 in order
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));
    assert!(sql.contains("$3"));
    assert_eq!(expr.binds().len(), 3);
}

#[test]
fn expr_or_binds_and_count() {
    let c1 = Column::new("users", "role");
    let c2 = Column::new("users", "role");
    let expr = c1.eq("admin").or(c2.eq("mod"));
    assert_eq!(expr.binds().len(), 2);
    assert_eq!(
        expr.binds(),
        vec![Value::Text("admin".into()), Value::Text("mod".into())]
    );
}

#[test]
fn expr_is_null_bind_count() {
    let col = Column::new("users", "deleted_at");
    let expr = col.is_null();
    assert_eq!(expr.bind_count(), 0);
}

#[test]
fn expr_in_list_combined_with_and() {
    let c1 = Column::new("users", "status");
    let c2 = Column::new("users", "active");
    let expr = c1
        .in_list(vec![Value::from("a"), Value::from("b")])
        .and(c2.eq(true));
    let sql = expr.to_sql(1);
    assert_eq!(
        sql,
        "(\"users\".\"status\" IN ($1, $2) AND \"users\".\"active\" = $3)"
    );
    assert_eq!(expr.binds().len(), 3);
}
