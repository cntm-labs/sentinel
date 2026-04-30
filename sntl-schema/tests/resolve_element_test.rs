use sntl_schema::cache::{CacheEntry, ColumnInfo, ElementTypeRef, QueryKind};
use sntl_schema::resolve::{ResolveInput, resolve_offline};
use sntl_schema::schema::{Column, PgTypeRef, Schema, Table};

fn schema_with_users() -> Schema {
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
                    pg_type: PgTypeRef::simple("int4"),
                    oid: 23,
                    nullable: false,
                    primary_key: true,
                    unique: false,
                    default: None,
                },
                Column {
                    name: "tags".into(),
                    pg_type: PgTypeRef::simple("_text"),
                    oid: 1009,
                    nullable: false,
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

fn entry_with_tags() -> CacheEntry {
    CacheEntry {
        version: 1,
        sql_hash: "tags1".into(),
        sql_normalized: "SELECT id, tags FROM users".into(),
        source_locations: vec![],
        params: vec![],
        columns: vec![
            ColumnInfo {
                name: "id".into(),
                pg_type: "int4".into(),
                oid: 23,
                nullable: false,
                origin: None,
                element_type: None,
            },
            ColumnInfo {
                name: "tags".into(),
                pg_type: "_text".into(),
                oid: 1009,
                nullable: false,
                origin: None,
                element_type: Some(ElementTypeRef {
                    pg_type: "text".into(),
                    oid: 25,
                }),
            },
        ],
        query_kind: QueryKind::Select,
        has_returning: false,
    }
}

#[test]
fn override_passes_through_when_column_is_array() {
    let schema = schema_with_users();
    let entry = entry_with_tags();
    let r = resolve_offline(ResolveInput {
        sql: "SELECT id, tags FROM users",
        cache_entry: &entry,
        schema: &schema,
        overrides_nullable: &[],
        overrides_non_null: &[],
        overrides_non_null_elements: &["tags".to_string()],
        strict: true,
    })
    .unwrap();
    assert_eq!(r.non_null_elements, vec!["tags".to_string()]);
}

#[test]
fn rejects_override_on_non_array_column() {
    let schema = schema_with_users();
    let entry = entry_with_tags();
    let err = resolve_offline(ResolveInput {
        sql: "SELECT id, tags FROM users",
        cache_entry: &entry,
        schema: &schema,
        overrides_nullable: &[],
        overrides_non_null: &[],
        overrides_non_null_elements: &["id".to_string()],
        strict: true,
    })
    .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("not an array column"), "{msg}");
}

#[test]
fn rejects_override_on_unknown_column() {
    let schema = schema_with_users();
    let entry = entry_with_tags();
    let err = resolve_offline(ResolveInput {
        sql: "SELECT id, tags FROM users",
        cache_entry: &entry,
        schema: &schema,
        overrides_nullable: &[],
        overrides_non_null: &[],
        overrides_non_null_elements: &["bogus".to_string()],
        strict: true,
    })
    .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("unknown column"), "{msg}");
}
