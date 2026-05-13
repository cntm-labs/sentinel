//! End-to-end test for `sntl migrate add → run → info`. Skips without DATABASE_URL.

use std::process::Command;

fn cli() -> &'static str {
    env!("CARGO_BIN_EXE_sntl")
}

async fn drop_setup(url: &str, stmts: &[&str]) {
    let cfg = sentinel_driver::Config::parse(url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(cfg).await.unwrap();
    conn.execute("SET client_min_messages = ERROR", &[])
        .await
        .unwrap();
    for s in stmts {
        conn.execute(s, &[]).await.unwrap();
    }
}

#[tokio::test]
async fn add_then_run_then_info() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else {
        return;
    };
    drop_setup(
        &url,
        &["DROP TABLE IF EXISTS _sntl_migrations, cli_e2e_test"],
    )
    .await;
    let dir = tempfile::tempdir().unwrap();

    // add
    let out = Command::new(cli())
        .args([
            "--workspace",
            &dir.path().to_string_lossy(),
            "migrate",
            "add",
            "create cli e2e",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Replace up.sql with real SQL
    let folder = std::fs::read_dir(dir.path().join("migrations"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    std::fs::write(
        folder.path().join("up.sql"),
        "CREATE TABLE cli_e2e_test (id int);",
    )
    .unwrap();

    // run (skip-refresh: we don't need cache for this test)
    let out = Command::new(cli())
        .args([
            "--workspace",
            &dir.path().to_string_lossy(),
            "--database-url",
            &url,
            "migrate",
            "run",
            "--skip-refresh",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // info shows it applied
    let out = Command::new(cli())
        .args([
            "--workspace",
            &dir.path().to_string_lossy(),
            "--database-url",
            &url,
            "migrate",
            "info",
            "--all",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "info failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("✓"), "stdout was: {stdout}");

    // verify reports clean
    let out = Command::new(cli())
        .args([
            "--workspace",
            &dir.path().to_string_lossy(),
            "--database-url",
            &url,
            "migrate",
            "verify",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "verify failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // mutate the migration file → verify should now fail
    std::fs::write(
        folder.path().join("up.sql"),
        "CREATE TABLE cli_e2e_test (id int); -- drifted",
    )
    .unwrap();
    let out = Command::new(cli())
        .args([
            "--workspace",
            &dir.path().to_string_lossy(),
            "--database-url",
            &url,
            "migrate",
            "verify",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "verify should detect drift");
}
