use anyhow::Result;
use std::path::PathBuf;

pub async fn run(
    _workspace: Option<PathBuf>,
    _database_url: Option<String>,
    _check: bool,
) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
