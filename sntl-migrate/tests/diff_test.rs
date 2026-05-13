use sntl_migrate::diff::{Change, compare, emit};
use sntl_schema::schema::{Column, PgTypeRef, Schema, Table};

fn col(name: &str, ty: &str, nullable: bool) -> Column {
    Column {
        name: name.into(),
        pg_type: PgTypeRef::simple(ty),
        oid: 0,
        nullable,
        primary_key: false,
        unique: false,
        default: None,
    }
}

fn tbl(name: &str, cols: Vec<Column>) -> Table {
    Table {
        name: name.into(),
        schema: "public".into(),
        columns: cols,
        foreign_keys: vec![],
    }
}

fn sch(tables: Vec<Table>) -> Schema {
    Schema {
        version: 1,
        postgres_version: "16".into(),
        generated_at: None,
        source: None,
        tables,
        enums: vec![],
        composites: vec![],
    }
}

#[test]
fn add_table_change() {
    let cache = sch(vec![tbl("users", vec![col("id", "int4", false)])]);
    let live = sch(vec![]);
    let changes = compare(&cache, &live);
    assert!(matches!(changes[0], Change::AddTable(_)));
}

#[test]
fn drop_table_change() {
    let cache = sch(vec![]);
    let live = sch(vec![tbl("users", vec![col("id", "int4", false)])]);
    let changes = compare(&cache, &live);
    assert!(matches!(&changes[0], Change::DropTable { name } if name == "users"));
}

#[test]
fn add_column_change() {
    let cache = sch(vec![tbl(
        "u",
        vec![col("id", "int4", false), col("name", "text", false)],
    )]);
    let live = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let changes = compare(&cache, &live);
    assert!(
        changes
            .iter()
            .any(|c| matches!(c, Change::AddColumn { table, .. } if table == "u"))
    );
}

#[test]
fn alter_type_change() {
    let cache = sch(vec![tbl("u", vec![col("id", "int8", false)])]);
    let live = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let changes = compare(&cache, &live);
    assert!(matches!(
        &changes[0],
        Change::AlterColumnType { from, to, .. } if from == "int4" && to == "int8"
    ));
}

#[test]
fn emit_add_table_clean() {
    let cache = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let changes = compare(&cache, &sch(vec![]));
    let (out, todos) = emit(&changes);
    assert!(out.contains("CREATE TABLE u"));
    assert_eq!(todos, 0);
}

#[test]
fn emit_drop_table_is_todo() {
    let live = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let changes = compare(&sch(vec![]), &live);
    let (out, todos) = emit(&changes);
    assert_eq!(todos, 1);
    assert!(out.contains("TODO: confirm DROP"));
}

#[test]
fn emit_widening_alter_is_clean() {
    let cache = sch(vec![tbl("u", vec![col("id", "int8", false)])]);
    let live = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let (out, todos) = emit(&compare(&cache, &live));
    assert_eq!(todos, 0);
    assert!(out.contains("ALTER TABLE u ALTER COLUMN id TYPE int8"));
}

#[test]
fn emit_narrowing_alter_has_todo() {
    let cache = sch(vec![tbl("u", vec![col("id", "int4", false)])]);
    let live = sch(vec![tbl("u", vec![col("id", "int8", false)])]);
    let (out, todos) = emit(&compare(&cache, &live));
    assert_eq!(todos, 1);
    assert!(out.contains("USING id::int4"));
}
