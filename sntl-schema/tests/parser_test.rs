use sntl_schema::parser::{parse_statement, ParsedStatement};

#[test]
fn parses_simple_select() {
    let stmt = parse_statement("SELECT id FROM users WHERE id = $1").unwrap();
    assert!(matches!(stmt, ParsedStatement::Select(_)));
}

#[test]
fn parses_insert_returning() {
    let stmt = parse_statement("INSERT INTO users (email) VALUES ($1) RETURNING id").unwrap();
    assert!(matches!(stmt, ParsedStatement::Insert { .. }));
}

#[test]
fn parses_update_returning() {
    let stmt = parse_statement("UPDATE users SET email = $1 WHERE id = $2 RETURNING id").unwrap();
    assert!(matches!(stmt, ParsedStatement::Update { .. }));
}

#[test]
fn rejects_garbage() {
    assert!(parse_statement("not sql at all").is_err());
}
