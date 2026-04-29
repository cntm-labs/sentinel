use sntl_schema::cache::{Cache, CacheEntry, ColumnInfo, ElementTypeRef, QueryKind};
use tempfile::tempdir;

fn entry_with_array_column() -> CacheEntry {
    CacheEntry {
        version: 1,
        sql_hash: "arr1".into(),
        sql_normalized: "SELECT tags FROM users".into(),
        source_locations: vec![],
        params: vec![],
        columns: vec![ColumnInfo {
            name: "tags".into(),
            pg_type: "_text".into(),
            oid: 1009,
            nullable: false,
            origin: None,
            element_type: Some(ElementTypeRef {
                pg_type: "text".into(),
                oid: 25,
            }),
        }],
        query_kind: QueryKind::Select,
        has_returning: false,
    }
}

#[test]
fn element_type_roundtrip() {
    let dir = tempdir().unwrap();
    let cache = Cache::new(dir.path());
    cache.init().unwrap();
    let entry = entry_with_array_column();
    cache.write_entry(&entry).unwrap();
    let loaded = cache.read_entry("arr1").unwrap();
    assert_eq!(
        loaded.columns[0].element_type,
        Some(ElementTypeRef {
            pg_type: "text".into(),
            oid: 25,
        })
    );
}

#[test]
fn old_cache_without_element_type_deserialises() {
    let dir = tempdir().unwrap();
    let cache = Cache::new(dir.path());
    cache.init().unwrap();

    // Hand-write a v1 cache file from before this field existed.
    let path = dir.path().join("queries").join("legacy.json");
    std::fs::write(
        &path,
        r#"{
            "version": 1,
            "sql_hash": "legacy",
            "sql_normalized": "SELECT id FROM users",
            "params": [],
            "columns": [{
                "name": "id",
                "pg_type": "int4",
                "oid": 23,
                "nullable": false
            }],
            "query_kind": "Select",
            "has_returning": false
        }"#,
    )
    .unwrap();

    let loaded = cache.read_entry("legacy").unwrap();
    assert!(
        loaded.columns[0].element_type.is_none(),
        "missing field must default to None for backward compat"
    );
}
