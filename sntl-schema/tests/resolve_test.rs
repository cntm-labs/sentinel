use sntl_schema::cache::CacheEntry;
use sntl_schema::resolve::{resolve_offline, ResolveInput};
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
                    pg_type: PgTypeRef::simple("uuid"),
                    oid: 2950,
                    nullable: false,
                    primary_key: true,
                    unique: false,
                    default: None,
                },
                Column {
                    name: "email".into(),
                    pg_type: PgTypeRef::simple("text"),
                    oid: 25,
                    nullable: false,
                    primary_key: false,
                    unique: true,
                    default: None,
                },
            ],
            foreign_keys: vec![],
        }],
        enums: vec![],
        composites: vec![],
    }
}

#[test]
fn resolves_simple_select_from_cache_entry() {
    let cache_entry = CacheEntry {
        version: 1,
        sql_hash: "abc".into(),
        sql_normalized: "SELECT id, email FROM users WHERE id = $1".into(),
        source_locations: vec![],
        params: vec![sntl_schema::cache::ParamInfo {
            index: 1,
            pg_type: "uuid".into(),
            oid: 2950,
        }],
        columns: vec![
            sntl_schema::cache::ColumnInfo {
                name: "id".into(),
                pg_type: "uuid".into(),
                oid: 2950,
                nullable: false,
                origin: Some(sntl_schema::cache::ColumnOrigin {
                    table: "users".into(),
                    column: "id".into(),
                }),
            },
            sntl_schema::cache::ColumnInfo {
                name: "email".into(),
                pg_type: "text".into(),
                oid: 25,
                nullable: false,
                origin: Some(sntl_schema::cache::ColumnOrigin {
                    table: "users".into(),
                    column: "email".into(),
                }),
            },
        ],
        query_kind: sntl_schema::cache::QueryKind::Select,
        has_returning: false,
    };
    let schema = schema_with_users();
    let input = ResolveInput {
        sql: "SELECT id, email FROM users WHERE id = $1",
        cache_entry: &cache_entry,
        schema: &schema,
        overrides_nullable: &[],
        overrides_non_null: &[],
        strict: true,
    };
    let r = resolve_offline(input).unwrap();
    assert_eq!(r.columns.len(), 2);
    assert!(!r.columns[0].nullable);
}

#[test]
fn override_nullable_is_applied() {
    let cache_entry = CacheEntry {
        version: 1,
        sql_hash: "abc".into(),
        sql_normalized: "SELECT id FROM users".into(),
        source_locations: vec![],
        params: vec![],
        columns: vec![sntl_schema::cache::ColumnInfo {
            name: "id".into(),
            pg_type: "uuid".into(),
            oid: 2950,
            nullable: false,
            origin: None,
        }],
        query_kind: sntl_schema::cache::QueryKind::Select,
        has_returning: false,
    };
    let schema = schema_with_users();
    let input = ResolveInput {
        sql: "SELECT id FROM users",
        cache_entry: &cache_entry,
        schema: &schema,
        overrides_nullable: &["id".to_string()],
        overrides_non_null: &[],
        strict: true,
    };
    let r = resolve_offline(input).unwrap();
    assert!(r.columns[0].nullable);
}
