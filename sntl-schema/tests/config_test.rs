use sntl_schema::config::{Config, OfflineMode};
use std::sync::Mutex;
use tempfile::tempdir;

// Env-var based tests share process state and must not interleave. Tests that
// touch SENTINEL_* take this lock first; tests that check defaults take it too
// so a preceding test's state cannot leak in.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn loads_minimal_config() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    unsafe { std::env::remove_var("SENTINEL_DATABASE_URL") };
    unsafe { std::env::remove_var("SENTINEL_OFFLINE") };

    let dir = tempdir().unwrap();
    let path = dir.path().join("sentinel.toml");
    std::fs::write(
        &path,
        r#"
[database]
url = "postgres://localhost/app_dev"
"#,
    )
    .unwrap();

    let cfg = Config::load_from(&path).unwrap();
    assert_eq!(
        cfg.database.url.as_deref(),
        Some("postgres://localhost/app_dev")
    );
    assert_eq!(cfg.offline.enabled, OfflineMode::Off);
}

#[test]
fn env_overrides_database_url() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    unsafe { std::env::set_var("SENTINEL_DATABASE_URL", "postgres://from-env/db") };
    let dir = tempdir().unwrap();
    let path = dir.path().join("sentinel.toml");
    std::fs::write(&path, "[database]\n").unwrap();

    let cfg = Config::load_from(&path).unwrap();
    assert_eq!(cfg.database.url.as_deref(), Some("postgres://from-env/db"));
    unsafe { std::env::remove_var("SENTINEL_DATABASE_URL") };
}

#[test]
fn defaults_when_file_missing() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    unsafe { std::env::remove_var("SENTINEL_DATABASE_URL") };
    unsafe { std::env::remove_var("SENTINEL_OFFLINE") };

    let cfg = Config::load_from("/nonexistent/path.toml").unwrap();
    assert!(cfg.database.url.is_none());
    assert_eq!(cfg.offline.enabled, OfflineMode::Off);
    assert_eq!(cfg.cache.dir, ".sentinel");
}

#[test]
fn env_offline_enables_offline_mode() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    unsafe { std::env::set_var("SENTINEL_OFFLINE", "true") };
    let cfg = Config::load_from("/nonexistent.toml").unwrap();
    assert_eq!(cfg.offline.enabled, OfflineMode::On);
    unsafe { std::env::remove_var("SENTINEL_OFFLINE") };
}
