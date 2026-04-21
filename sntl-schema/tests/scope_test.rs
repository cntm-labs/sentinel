use sntl_schema::parser::{parse_statement, ParsedStatement};
use sntl_schema::scope::{build_scope, JoinKind};

fn as_select(stmt: ParsedStatement) -> sqlparser::ast::Query {
    match stmt {
        ParsedStatement::Select(q) => *q,
        _ => panic!("not a select"),
    }
}

#[test]
fn single_table_scope() {
    let q = as_select(parse_statement("SELECT id FROM users").unwrap());
    let scope = build_scope(&q).unwrap();
    assert_eq!(scope.tables.len(), 1);
    assert_eq!(scope.tables[0].alias, "users");
    assert_eq!(scope.tables[0].table_name, "users");
    assert_eq!(scope.tables[0].join_kind, JoinKind::Base);
}

#[test]
fn left_join_marks_right_as_forced_nullable() {
    let q = as_select(
        parse_statement("SELECT * FROM users u LEFT JOIN posts p ON p.user_id = u.id").unwrap(),
    );
    let scope = build_scope(&q).unwrap();
    let posts = scope.tables.iter().find(|t| t.alias == "p").unwrap();
    assert_eq!(posts.table_name, "posts");
    assert_eq!(posts.join_kind, JoinKind::LeftForcedNullable);
}

#[test]
fn alias_is_tracked() {
    let q = as_select(parse_statement("SELECT u.id FROM users AS u").unwrap());
    let scope = build_scope(&q).unwrap();
    let t = scope.resolve_alias("u").unwrap();
    assert_eq!(t.table_name, "users");
}
