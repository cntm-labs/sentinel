//! Apply fixture SQL files in declared order, inside one transaction.

use std::path::PathBuf;

use driver::Connection;

/// Apply each named fixture (e.g. "users" → `<fixtures_dir>/users.sql`)
/// inside a single transaction. Rolls back on error.
pub async fn apply(
    conn: &mut Connection,
    fixtures_dir: &std::path::Path,
    names: &[&str],
) -> anyhow::Result<()> {
    if names.is_empty() {
        return Ok(());
    }

    conn.execute("BEGIN", &[]).await?;
    for name in names {
        let mut p: PathBuf = fixtures_dir.to_path_buf();
        p.push(format!("{name}.sql"));
        let sql = std::fs::read_to_string(&p)
            .map_err(|e| anyhow::anyhow!("read fixture {}: {}", p.display(), e))?;
        if let Err(e) = conn.execute(&sql, &[]).await {
            conn.execute("ROLLBACK", &[]).await.ok();
            return Err(e.into());
        }
    }
    conn.execute("COMMIT", &[]).await?;
    Ok(())
}
