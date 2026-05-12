use sntl_migrate::discover::discover;
use std::fs;
use tempfile::tempdir;

fn touch(dir: &std::path::Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[test]
fn empty_dir_returns_empty_vec() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("migrations")).unwrap();
    let m = discover(&dir.path().join("migrations")).unwrap();
    assert!(m.is_empty());
}

#[test]
fn finds_and_sorts_two_migrations() {
    let dir = tempdir().unwrap();
    touch(
        dir.path(),
        "migrations/20260510_080000_b/up.sql",
        "SELECT 2;",
    );
    touch(
        dir.path(),
        "migrations/20260509_140000_a/up.sql",
        "SELECT 1;",
    );
    let m = discover(&dir.path().join("migrations")).unwrap();
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].version.name(), "a");
    assert_eq!(m[1].version.name(), "b");
}

#[test]
fn detects_up_notx_variant() {
    let dir = tempdir().unwrap();
    touch(
        dir.path(),
        "migrations/20260509_140000_idx/up.notx.sql",
        "CREATE INDEX CONCURRENTLY ...",
    );
    let m = discover(&dir.path().join("migrations")).unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].tx_mode, sntl_migrate::migration::TxMode::None);
}

#[test]
fn rejects_malformed_folder() {
    let dir = tempdir().unwrap();
    touch(dir.path(), "migrations/not_a_version/up.sql", "");
    let err = discover(&dir.path().join("migrations")).unwrap_err();
    assert!(matches!(err, sntl_migrate::error::Error::InvalidName { .. }));
}

#[test]
fn missing_dir_returns_missing_error() {
    let dir = tempdir().unwrap();
    let err = discover(&dir.path().join("nope")).unwrap_err();
    assert!(matches!(err, sntl_migrate::error::Error::MissingDir { .. }));
}
