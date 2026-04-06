use sntl::core::query::InsertQuery;
use sntl::core::types::Value;

#[test]
fn insert_single_row() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .column("name", "Alice");
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\", \"name\") VALUES ($1, $2) RETURNING *"
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0], Value::Text("alice@example.com".into()));
    assert_eq!(binds[1], Value::Text("Alice".into()));
}

#[test]
fn insert_with_returning_specific() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .returning(vec!["id", "email"]);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\") VALUES ($1) RETURNING \"id\", \"email\""
    );
}

#[test]
fn insert_with_no_returning() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .no_returning();
    let (sql, _) = q.build();
    assert_eq!(sql, "INSERT INTO \"users\" (\"email\") VALUES ($1)");
}

#[test]
fn insert_on_conflict_do_nothing() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .on_conflict_do_nothing("email");
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\") VALUES ($1) \
         ON CONFLICT (\"email\") DO NOTHING RETURNING *"
    );
}

#[test]
fn insert_multiple_values() {
    let q = InsertQuery::new("users")
        .column("email", "alice@example.com")
        .column("name", "Alice")
        .column("active", true);
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\", \"name\", \"active\") VALUES ($1, $2, $3) RETURNING *"
    );
    assert_eq!(binds.len(), 3);
}
