use std::path::Path;

use crate::error::Result;

/// Pull the live schema and write it to `<cache_dir>/schema.toml`.
///
/// Called by `Migrator::run` after migrations apply so the compile-time
/// `query!()` cache always reflects the new schema. The caller controls
/// the cache directory.
pub async fn refresh_schema(conn_str: &str, cache_dir: &Path) -> Result<()> {
    let schema = sntl_schema::introspect::pull_schema(conn_str).await?;
    let cache = sntl_schema::cache::Cache::new(cache_dir);
    cache.init()?;
    cache.write_schema(&schema)?;
    Ok(())
}
