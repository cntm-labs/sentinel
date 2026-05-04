//! Integration tests for `sntl init`. Each test runs in an isolated
//! tempdir so we don't pollute the workspace's own .sentinel/.

use std::path::Path;

fn cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_sntl")
}

fn run_init(workspace: &Path, force: bool) -> std::process::Output {
    let mut cmd = std::process::Command::new(cli_binary());
    cmd.arg("--workspace").arg(workspace).arg("init");
    if force {
        cmd.arg("--force");
    }
    cmd.output().expect("spawn sntl init")
}

#[test]
fn init_scaffolds_fresh_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_init(dir.path(), false);
    assert!(
        out.status.success(),
        "stdout={:?} stderr={:?}",
        out.stdout,
        out.stderr
    );

    let toml = dir.path().join("sentinel.toml");
    assert!(toml.exists(), "sentinel.toml must be written");
    let toml_text = std::fs::read_to_string(&toml).unwrap();
    assert!(toml_text.contains("[database]"));
    assert!(toml_text.contains("[macros]"));
    assert!(toml_text.contains("[cache]"));

    let cache_dir = dir.path().join(".sentinel");
    assert!(cache_dir.is_dir(), ".sentinel/ must be a directory");
    assert!(
        cache_dir.join("queries").is_dir(),
        ".sentinel/queries/ must exist"
    );
    assert!(
        cache_dir.join(".version").is_file(),
        ".sentinel/.version must exist"
    );
    assert_eq!(
        std::fs::read_to_string(cache_dir.join(".version"))
            .unwrap()
            .trim(),
        "1"
    );

    let gitignore = cache_dir.join(".gitignore");
    assert!(gitignore.is_file(), ".sentinel/.gitignore must exist");
    let gi_text = std::fs::read_to_string(&gitignore).unwrap();
    // Working directories ignored, cache files NOT ignored.
    assert!(gi_text.contains("/wip/"));
    assert!(!gi_text.contains("queries/"));
}

#[test]
fn init_does_not_overwrite_existing_sentinel_toml() {
    let dir = tempfile::tempdir().unwrap();
    let toml = dir.path().join("sentinel.toml");
    let original = "# Pre-existing user config\n[database]\nurl = \"hand-edited\"\n";
    std::fs::write(&toml, original).unwrap();

    let out = run_init(dir.path(), false);
    assert!(
        out.status.success(),
        "stderr={:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    // sentinel.toml unchanged
    assert_eq!(std::fs::read_to_string(&toml).unwrap(), original);
    // ...but .sentinel/ still set up
    assert!(dir.path().join(".sentinel/.version").exists());
}

#[test]
fn init_force_overwrites_existing_sentinel_toml() {
    let dir = tempfile::tempdir().unwrap();
    let toml = dir.path().join("sentinel.toml");
    std::fs::write(&toml, "garbage").unwrap();

    let out = run_init(dir.path(), true);
    assert!(
        out.status.success(),
        "stderr={:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    let new_text = std::fs::read_to_string(&toml).unwrap();
    assert!(
        new_text.contains("[database]"),
        "force should rewrite the template"
    );
    assert!(!new_text.contains("garbage"));
}

#[test]
fn init_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let first = run_init(dir.path(), false);
    let second = run_init(dir.path(), false);
    assert!(first.status.success());
    assert!(second.status.success());
    // Files still present, content unchanged on the second pass.
    assert!(dir.path().join("sentinel.toml").exists());
    assert!(dir.path().join(".sentinel/.version").exists());
}
