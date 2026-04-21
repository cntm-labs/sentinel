use sntl_schema::schema::{Column, PgTypeRef, Schema, Table};

#[test]
fn parses_schema_toml() {
    let toml = r#"
version = 1
postgres_version = "16.2"
generated_at = "2026-04-20T10:30:00Z"
source = "postgres://localhost:5432/myapp_dev"

[[tables]]
name = "users"
schema = "public"

  [[tables.columns]]
  name = "id"
  pg_type = "uuid"
  oid = 2950
  nullable = false
  primary_key = true

  [[tables.columns]]
  name = "email"
  pg_type = "text"
  oid = 25
  nullable = false
  unique = true
"#;
    let schema: Schema = toml::from_str(toml).unwrap();
    assert_eq!(schema.version, 1);
    assert_eq!(schema.tables.len(), 1);
    assert_eq!(schema.tables[0].name, "users");
    assert_eq!(schema.tables[0].columns.len(), 2);
    assert!(schema.tables[0].columns[0].primary_key);
    assert!(!schema.tables[0].columns[1].nullable);
}

#[test]
fn lookup_table_and_column() {
    let t = Table {
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
    };
    let s = Schema {
        version: 1,
        postgres_version: "16".into(),
        generated_at: None,
        source: None,
        tables: vec![t],
        enums: vec![],
        composites: vec![],
    };
    assert!(s.find_table("users").is_some());
    assert!(s.find_column("users", "email").is_some());
    assert!(s.find_column("users", "missing").is_none());
}
