use sntl_schema::nullable::{infer_expr_nullability, ExprContext};
use sntl_schema::scope::{JoinKind, Scope, TableRef};
use sntl_schema::schema::{Column, PgTypeRef, Schema, Table};

fn simple_schema() -> Schema {
    Schema {
        version: 1,
        postgres_version: "16".into(),
        generated_at: None,
        source: None,
        tables: vec![Table {
            name: "users".into(),
            schema: "public".into(),
            columns: vec![
                Column {
                    name: "id".into(),
                    pg_type: PgTypeRef::simple("uuid"),
                    oid: 2950,
                    nullable: false,
                    primary_key: true,
                    unique: false,
                    default: None,
                },
                Column {
                    name: "deleted_at".into(),
                    pg_type: PgTypeRef::simple("timestamptz"),
                    oid: 1184,
                    nullable: true,
                    primary_key: false,
                    unique: false,
                    default: None,
                },
            ],
            foreign_keys: vec![],
        }],
        enums: vec![],
        composites: vec![],
    }
}

fn simple_scope() -> Scope {
    Scope {
        tables: vec![TableRef {
            alias: "users".into(),
            table_name: "users".into(),
            schema: None,
            join_kind: JoinKind::Base,
        }],
    }
}

#[test]
fn column_nullable_from_schema() {
    let schema = simple_schema();
    let scope = simple_scope();
    let ctx = ExprContext {
        schema: &schema,
        scope: &scope,
        strict: true,
    };
    let parsed = sqlparser::parser::Parser::parse_sql(
        &sqlparser::dialect::PostgreSqlDialect {},
        "SELECT deleted_at FROM users",
    )
    .unwrap();
    let body = if let sqlparser::ast::Statement::Query(q) = &parsed[0] {
        q
    } else {
        panic!()
    };
    let select = if let sqlparser::ast::SetExpr::Select(s) = &*body.body {
        s
    } else {
        panic!()
    };
    let sel_item = &select.projection[0];
    let expr = match sel_item {
        sqlparser::ast::SelectItem::UnnamedExpr(e) => e,
        _ => panic!(),
    };
    assert!(infer_expr_nullability(expr, &ctx).nullable);
}

#[test]
fn coalesce_non_null_if_any_non_null() {
    // `COALESCE(deleted_at, '1970-01-01')` → non-null
    let schema = simple_schema();
    let scope = simple_scope();
    let ctx = ExprContext {
        schema: &schema,
        scope: &scope,
        strict: true,
    };
    let parsed = sqlparser::parser::Parser::parse_sql(
        &sqlparser::dialect::PostgreSqlDialect {},
        "SELECT COALESCE(deleted_at, '1970-01-01'::timestamptz) FROM users",
    )
    .unwrap();
    let body = if let sqlparser::ast::Statement::Query(q) = &parsed[0] {
        q
    } else {
        panic!()
    };
    let select = if let sqlparser::ast::SetExpr::Select(s) = &*body.body {
        s
    } else {
        panic!()
    };
    let expr = match &select.projection[0] {
        sqlparser::ast::SelectItem::UnnamedExpr(e) => e,
        _ => panic!(),
    };
    assert!(!infer_expr_nullability(expr, &ctx).nullable);
}

#[test]
fn literal_null_is_nullable() {
    let schema = simple_schema();
    let scope = simple_scope();
    let ctx = ExprContext {
        schema: &schema,
        scope: &scope,
        strict: true,
    };
    let parsed = sqlparser::parser::Parser::parse_sql(
        &sqlparser::dialect::PostgreSqlDialect {},
        "SELECT NULL FROM users",
    )
    .unwrap();
    let body = if let sqlparser::ast::Statement::Query(q) = &parsed[0] {
        q
    } else {
        panic!()
    };
    let select = if let sqlparser::ast::SetExpr::Select(s) = &*body.body {
        s
    } else {
        panic!()
    };
    let expr = match &select.projection[0] {
        sqlparser::ast::SelectItem::UnnamedExpr(e) => e,
        _ => panic!(),
    };
    assert!(infer_expr_nullability(expr, &ctx).nullable);
}
