//! Black-box test for `sntl migrate add` — drives the compiled binary.

use std::path::Path;

fn cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_sntl")
}

#[test]
fn add_creates_folder_and_up_sql() {
    let dir = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(cli_binary())
        .arg("--workspace")
        .arg(dir.path())
        .arg("migrate")
        .arg("add")
        .arg("add users")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mig_dir = dir.path().join("migrations");
    assert!(mig_dir.is_dir(), "migrations/ was not created");
    let inside: Vec<_> = std::fs::read_dir(&mig_dir).unwrap().flatten().collect();
    assert_eq!(inside.len(), 1);
    let folder_name = inside[0].file_name().to_string_lossy().into_owned();
    assert!(folder_name.ends_with("_add_users"), "got {folder_name}");
    assert!(Path::new(&inside[0].path().join("up.sql")).exists());
}

#[test]
fn add_refuses_when_no_create_dir() {
    let dir = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(cli_binary())
        .arg("--workspace")
        .arg(dir.path())
        .arg("migrate")
        .arg("add")
        .arg("--no-create-dir")
        .arg("foo")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "should fail without migrations/ present"
    );
    assert!(!dir.path().join("migrations").exists());
}
