use sntl_schema::cache::{Cache, CacheEntry, ColumnInfo, ColumnOrigin, ParamInfo, QueryKind};
use tempfile::tempdir;

fn sample_entry() -> CacheEntry {
    CacheEntry {
        version: 1,
        sql_hash: "a3f7c2e9b1d4a".into(),
        sql_normalized: "SELECT id FROM users WHERE id = $1".into(),
        source_locations: vec![],
        params: vec![ParamInfo {
            index: 1,
            pg_type: "uuid".into(),
            oid: 2950,
        }],
        columns: vec![ColumnInfo {
            name: "id".into(),
            pg_type: "uuid".into(),
            oid: 2950,
            nullable: false,
            origin: Some(ColumnOrigin {
                table: "users".into(),
                column: "id".into(),
            }),
            element_type: None,
        }],
        query_kind: QueryKind::Select,
        has_returning: false,
    }
}

#[test]
fn write_and_read_entry_roundtrip() {
    let dir = tempdir().unwrap();
    let cache = Cache::new(dir.path());
    cache.init().unwrap();
    let entry = sample_entry();
    cache.write_entry(&entry).unwrap();
    let loaded = cache.read_entry(&entry.sql_hash).unwrap();
    assert_eq!(loaded.sql_normalized, entry.sql_normalized);
    assert_eq!(loaded.columns.len(), 1);
}

#[test]
fn missing_entry_is_cache_miss() {
    let dir = tempdir().unwrap();
    let cache = Cache::new(dir.path());
    cache.init().unwrap();
    let err = cache.read_entry("does_not_exist").unwrap_err();
    assert!(matches!(err, sntl_schema::Error::CacheMiss { .. }));
}

#[test]
fn version_is_written_and_checked() {
    let dir = tempdir().unwrap();
    let cache = Cache::new(dir.path());
    cache.init().unwrap();
    assert_eq!(cache.read_version().unwrap(), 1);
    std::fs::write(dir.path().join(".version"), "99").unwrap();
    let err = cache.check_version().unwrap_err();
    assert!(matches!(err, sntl_schema::Error::CacheVersionTooNew { .. }));
}
