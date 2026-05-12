use sentinel_driver::Connection;

use crate::error::Result;
use crate::migration::Version;

const TABLE_NAME: &str = "_sntl_migrations";

/// Create the tracking table if it does not exist. Idempotent.
pub async fn ensure(conn: &mut Connection) -> Result<()> {
    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {TABLE_NAME} (\
                version    text PRIMARY KEY,\
                applied_at timestamptz NOT NULL DEFAULT now(),\
                checksum   text NOT NULL\
            )"
        ),
        &[],
    )
    .await?;
    Ok(())
}

/// Drop the tracking table. Test helper only.
#[doc(hidden)]
pub async fn drop_table(conn: &mut Connection) -> Result<()> {
    conn.execute(&format!("DROP TABLE IF EXISTS {TABLE_NAME}"), &[])
        .await?;
    Ok(())
}

/// Return all applied migrations as `(version, checksum)` ordered by version.
pub async fn applied(conn: &mut Connection) -> Result<Vec<(Version, String)>> {
    let rows = conn
        .query(
            &format!("SELECT version, checksum FROM {TABLE_NAME} ORDER BY version ASC"),
            &[],
        )
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let v: String = row.try_get(0)?;
        let cs: String = row.try_get(1)?;
        let version: Version = v.parse()?;
        out.push((version, cs));
    }
    Ok(out)
}

/// Insert a successfully-applied migration record.
pub async fn record(conn: &mut Connection, version: &Version, checksum: &str) -> Result<()> {
    let v = version.as_str().to_string();
    let cs = checksum.to_string();
    conn.execute(
        &format!("INSERT INTO {TABLE_NAME} (version, checksum) VALUES ($1, $2)"),
        &[&v, &cs],
    )
    .await?;
    Ok(())
}
