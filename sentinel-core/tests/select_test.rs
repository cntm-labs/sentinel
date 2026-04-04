use sentinel_core::expr::Column;
use sentinel_core::query::SelectQuery;

#[test]
fn select_all_from_table() {
    let q = SelectQuery::new("users");
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
    assert!(binds.is_empty());
}

#[test]
fn select_specific_columns() {
    let q = SelectQuery::new("users").columns(vec!["id", "email", "name"]);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"email\", \"users\".\"name\" FROM \"users\""
    );
}

#[test]
fn select_with_where() {
    let col = Column::new("users", "email");
    let q = SelectQuery::new("users").where_(col.eq("alice@example.com"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" WHERE \"users\".\"email\" = $1"
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn select_with_multiple_where() {
    let email = Column::new("users", "email");
    let active = Column::new("users", "active");
    let q = SelectQuery::new("users")
        .where_(email.eq("alice@example.com"))
        .where_(active.eq(true));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" WHERE (\"users\".\"email\" = $1 AND \"users\".\"active\" = $2)"
    );
    assert_eq!(binds.len(), 2);
}

#[test]
fn select_with_order_by() {
    let col = Column::new("users", "created_at");
    let q = SelectQuery::new("users").order_by(col.desc());
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" ORDER BY \"users\".\"created_at\" DESC"
    );
}

#[test]
fn select_with_limit_offset() {
    let q = SelectQuery::new("users").limit(20).offset(40);
    let (sql, _) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\" LIMIT 20 OFFSET 40");
}

#[test]
fn select_full_query() {
    let email = Column::new("users", "email");
    let created = Column::new("users", "created_at");
    let q = SelectQuery::new("users")
        .columns(vec!["id", "email"])
        .where_(email.like("%@example.com"))
        .order_by(created.desc())
        .limit(10)
        .offset(0);
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"email\" FROM \"users\" \
         WHERE \"users\".\"email\" LIKE $1 \
         ORDER BY \"users\".\"created_at\" DESC \
         LIMIT 10 OFFSET 0"
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn select_for_update() {
    let q = SelectQuery::new("accounts").for_update();
    let (sql, _) = q.build();
    assert_eq!(sql, "SELECT \"accounts\".* FROM \"accounts\" FOR UPDATE");
}
