use std::path::{Path, PathBuf};

use sentinel_driver::advisory_lock::PgAdvisoryLock;
use sentinel_driver::{Connection, Pool};

use crate::SNTL_MIGRATE_LOCK_ID;
use crate::checksum::sha256_of_sql;
use crate::discover::discover;
use crate::error::{Error, Result};
use crate::migration::{Migration, TxMode, Version};
use crate::tracking;

/// Result of a single `Migrator::run` invocation.
#[derive(Debug, Default)]
pub struct MigrationReport {
    pub applied: Vec<Version>,
}

/// One row in `sntl migrate info`.
#[derive(Debug)]
pub struct MigrationStatus {
    pub version: Version,
    pub state: State,
    pub checksum: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Applied,
    Pending,
    ChecksumDrift,
}

#[derive(Debug, Clone)]
pub struct RefreshConfig {
    pub conn_str: String,
    pub cache_dir: PathBuf,
}

/// Top-level migration runner.
pub struct Migrator {
    migrations: Vec<Migration>,
    source: MigrationSource,
    refresh: Option<RefreshConfig>,
}

#[derive(Debug)]
enum MigrationSource {
    Dir(PathBuf),
    Static,
}

impl Migrator {
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let migrations = discover(&path)?;
        Ok(Self {
            migrations,
            source: MigrationSource::Dir(path),
            refresh: None,
        })
    }

    pub fn from_static(entries: &'static [(&'static str, &'static str, TxMode)]) -> Self {
        let migrations = entries
            .iter()
            .map(|(v, sql, mode)| Migration {
                version: v
                    .parse()
                    .expect("compile-time migration version must be valid"),
                sql: (*sql).to_string(),
                tx_mode: *mode,
            })
            .collect();
        Self {
            migrations,
            source: MigrationSource::Static,
            refresh: None,
        }
    }

    pub fn with_refresh(
        mut self,
        conn_str: impl Into<String>,
        cache_dir: impl Into<PathBuf>,
    ) -> Self {
        self.refresh = Some(RefreshConfig {
            conn_str: conn_str.into(),
            cache_dir: cache_dir.into(),
        });
        self
    }

    pub async fn run(&self, pool: &Pool) -> Result<MigrationReport> {
        let mut conn = pool.acquire().await?;
        let lock = PgAdvisoryLock::new(SNTL_MIGRATE_LOCK_ID);
        let guard = lock.acquire(&mut conn).await?;

        let result = self.run_locked(&mut conn).await;

        guard.release(&mut conn).await?;
        let report = result?;

        if let Some(cfg) = &self.refresh {
            crate::refresh::refresh_schema(&cfg.conn_str, &cfg.cache_dir).await?;
        }
        Ok(report)
    }

    async fn run_locked(&self, conn: &mut Connection) -> Result<MigrationReport> {
        tracking::ensure(conn).await?;
        let applied = tracking::applied(conn).await?;
        let applied_set: std::collections::BTreeSet<Version> =
            applied.iter().map(|(v, _)| v.clone()).collect();
        let highest_applied = applied_set.iter().max().cloned();

        let mut report = MigrationReport::default();
        for m in &self.migrations {
            if applied_set.contains(&m.version) {
                continue;
            }
            if let Some(highest) = &highest_applied {
                if m.version < *highest {
                    return Err(Error::OutOfOrder {
                        pending: m.version.clone(),
                        highest_applied: highest.clone(),
                    });
                }
            }

            apply_one(conn, m).await?;
            tracking::record(conn, &m.version, &sha256_of_sql(&m.sql)).await?;
            report.applied.push(m.version.clone());
        }
        Ok(report)
    }

    pub async fn info(&self, pool: &Pool) -> Result<Vec<MigrationStatus>> {
        let mut conn = pool.acquire().await?;
        tracking::ensure(&mut conn).await?;
        let applied = tracking::applied(&mut conn).await?;
        let applied_map: std::collections::BTreeMap<Version, String> =
            applied.into_iter().collect();

        let mut out = Vec::with_capacity(self.migrations.len() + applied_map.len());
        for m in &self.migrations {
            if let Some(recorded) = applied_map.get(&m.version) {
                let current = sha256_of_sql(&m.sql);
                let state = if current == *recorded {
                    State::Applied
                } else {
                    State::ChecksumDrift
                };
                out.push(MigrationStatus {
                    version: m.version.clone(),
                    state,
                    checksum: Some(recorded.clone()),
                });
            } else {
                out.push(MigrationStatus {
                    version: m.version.clone(),
                    state: State::Pending,
                    checksum: None,
                });
            }
        }
        Ok(out)
    }

    pub fn migrations(&self) -> &[Migration] {
        &self.migrations
    }

    pub fn source_path(&self) -> Option<&Path> {
        match &self.source {
            MigrationSource::Dir(p) => Some(p.as_path()),
            MigrationSource::Static => None,
        }
    }
}

async fn apply_one(conn: &mut Connection, m: &Migration) -> Result<()> {
    match m.tx_mode {
        TxMode::PerMigration => {
            conn.execute("BEGIN", &[]).await?;
            if let Err(e) = conn.execute(&m.sql, &[]).await {
                conn.execute("ROLLBACK", &[]).await.ok();
                return Err(Error::ApplyFailed {
                    version: m.version.clone(),
                    source: e,
                });
            }
            conn.execute("COMMIT", &[])
                .await
                .map_err(|source| Error::ApplyFailed {
                    version: m.version.clone(),
                    source,
                })?;
        }
        TxMode::None => {
            conn.execute(&m.sql, &[])
                .await
                .map_err(|source| Error::ApplyFailed {
                    version: m.version.clone(),
                    source,
                })?;
        }
    }
    Ok(())
}
