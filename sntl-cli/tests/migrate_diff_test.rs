//! End-to-end test for `sntl migrate diff`. Skips without DATABASE_URL.

use std::process::Command;

fn cli() -> &'static str {
    env!("CARGO_BIN_EXE_sntl")
}

#[tokio::test]
async fn diff_emits_file_when_drift_exists() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();

    // Seed a fake cache schema with a table the live DB doesn't have.
    std::fs::create_dir_all(dir.path().join(".sentinel/queries")).unwrap();
    std::fs::write(dir.path().join(".sentinel/.version"), "1").unwrap();
    std::fs::write(
        dir.path().join(".sentinel/schema.toml"),
        r#"
version = 1
postgres_version = "16"

[[tables]]
name = "fake_diff_table"
schema = "public"

  [[tables.columns]]
  name = "id"
  pg_type = "int4"
  oid = 23
  nullable = false
  primary_key = true
"#,
    )
    .unwrap();

    let out = Command::new(cli())
        .args([
            "--workspace",
            &dir.path().to_string_lossy(),
            "--database-url",
            &url,
            "migrate",
            "diff",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "diff failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // diff should emit one migration folder under migrations/.
    let migrations: Vec<_> = std::fs::read_dir(dir.path().join("migrations"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(migrations.len(), 1, "expected one diff folder");

    let up = migrations[0].path().join("up.sql");
    let body = std::fs::read_to_string(&up).unwrap();
    assert!(
        body.contains("CREATE TABLE fake_diff_table"),
        "scaffold should contain CREATE TABLE; got:\n{body}"
    );
}
