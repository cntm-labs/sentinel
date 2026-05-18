//! Entry point called by the `#[sntl::test]` macro expansion.

use std::future::Future;

/// Configuration for one test invocation. The `#[sntl::test]` macro
/// constructs this directly with literal strings.
pub struct TestConfig {
    pub test_name: &'static str,
    pub migrations_dir: Option<&'static str>,
    pub fixtures: &'static [&'static str],
}

/// Run `body` against a fresh per-test database. Synchronous wrapper —
/// the macro will spawn `body` inside `tokio::runtime::Runtime::new`'s
/// `block_on`.
pub async fn run<F, Fut>(_cfg: TestConfig, _body: F)
where
    F: FnOnce(driver::Pool) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    // Filled in by Tasks 11-13.
    unimplemented!("Task 11: template DB lifecycle");
}
