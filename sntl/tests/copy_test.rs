use sntl::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[test]
fn copy_in_sql_generates_correct_statement() {
    let sql = sntl::core::copy::copy_in_sql::<User>();
    assert_eq!(
        sql,
        "COPY \"users\" (\"id\", \"name\", \"email\") FROM STDIN BINARY"
    );
}

#[test]
fn copy_in_sql_includes_all_columns() {
    let sql = sntl::core::copy::copy_in_sql::<User>();
    assert!(sql.contains("COPY"));
    assert!(sql.contains("\"users\""));
    assert!(sql.contains("\"id\""));
    assert!(sql.contains("\"name\""));
    assert!(sql.contains("\"email\""));
    assert!(sql.contains("FROM STDIN BINARY"));
}
