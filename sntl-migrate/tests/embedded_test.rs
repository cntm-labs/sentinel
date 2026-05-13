//! Smoke test that the `sntl_migrate::migrate!()` macro expands and the
//! resulting `Migrator` is well-formed. No DB is contacted.

#[test]
fn embedded_macro_compiles() {
    let migrator = sntl_migrate::migrate!("./tests/embedded_fixtures/migrations");
    let migrations = migrator.migrations();
    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].version.as_str(), "20260101_000000_seed");
    assert!(migrations[0].sql.contains("CREATE TABLE embedded_test_t"));
    assert_eq!(migrations[0].tx_mode, sntl_migrate::TxMode::PerMigration);
}
