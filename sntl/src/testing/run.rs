//! Entry point called by the `#[sntl::test]` macro expansion.

use std::future::Future;
use std::path::{Path, PathBuf};

use driver::pool::config::PoolConfig;
use driver::{Config, Pool};

use crate::testing::template::with_db;
use crate::testing::{fixtures, template};

/// Configuration for one test invocation. The `#[sntl::test]` macro
/// constructs this directly with literal strings.
pub struct TestConfig {
    pub test_name: &'static str,
    pub migrations_dir: Option<&'static str>,
    pub fixtures_dir: Option<&'static str>,
    pub fixtures: &'static [&'static str],
}

/// Run `body` against a fresh per-test database.
///
/// Steps:
/// 1. Reads `SNTL_TEST_DATABASE_URL` (fallback `DATABASE_URL`) — skips if
///    neither is set.
/// 2. Builds or retrieves the template DB (once per process).
/// 3. Clones a fresh per-test DB from the template.
/// 4. Builds a [`Pool`] against the test DB.
/// 5. Applies fixtures (if any).
/// 6. Runs `body`.
/// 7. On success: drops the per-test DB (unless `SNTL_TEST_KEEP_DBS` is set).
/// 8. On failure: leaks the DB for inspection and panics.
pub async fn run<F, Fut>(cfg: TestConfig, body: F)
where
    F: FnOnce(Pool) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let admin_url = match std::env::var("SNTL_TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
    {
        Some(u) => u,
        None => {
            eprintln!(
                "skip {}: set SNTL_TEST_DATABASE_URL or DATABASE_URL",
                cfg.test_name
            );
            return;
        }
    };

    let migrations_dir = cfg.migrations_dir.map(Path::new);
    let template_db = match template::build_or_get(&admin_url, migrations_dir).await {
        Ok(t) => t,
        Err(e) => {
            panic!("sntl::test: build template DB failed: {e}");
        }
    };

    let suffix = format!("{}_{}", sanitise(cfg.test_name), rand_suffix());
    let test_db = template::clone_into(&template_db, &suffix)
        .await
        .unwrap_or_else(|e| panic!("sntl::test: clone DB failed: {e}"));

    let test_url = with_db(&admin_url, &test_db);

    let test_cfg = Config::parse(&test_url).expect("valid test URL");
    let pool = Pool::new(test_cfg, PoolConfig::new().max_connections(4));

    if !cfg.fixtures.is_empty() {
        let fixtures_root = cfg.fixtures_dir.map(PathBuf::from).unwrap_or_else(|| {
            let mfd = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
            PathBuf::from(mfd).join("tests").join("fixtures")
        });
        let mut conn = pool.acquire().await.expect("acquire conn for fixtures");
        fixtures::apply(&mut conn, &fixtures_root, cfg.fixtures)
            .await
            .unwrap_or_else(|e| panic!("sntl::test: apply fixtures failed: {e}"));
    }

    let result = body(pool).await;

    let keep = std::env::var("SNTL_TEST_KEEP_DBS").is_ok();
    if let Err(e) = result {
        if !keep {
            eprintln!(
                "sntl::test: test {} failed, leaking DB {} for debug:",
                cfg.test_name, test_db
            );
            eprintln!("  connect with: psql {test_url}");
        }
        panic!("sntl::test {} failed: {e}", cfg.test_name);
    }

    if !keep {
        template::drop_db(&admin_url, &test_db).await;
    }
}

fn sanitise(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    format!("{micros:x}")
}
