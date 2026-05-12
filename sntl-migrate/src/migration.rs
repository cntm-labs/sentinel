use std::fmt;
use std::str::FromStr;

use crate::error::Error;

/// Transaction mode for a migration file.
///
/// Default `PerMigration` wraps each migration in `BEGIN/COMMIT`. Migrations
/// with non-transactional DDL (`CREATE INDEX CONCURRENTLY`, `VACUUM`, etc.)
/// can declare `up.notx.sql` instead of `up.sql`, which maps to `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxMode {
    PerMigration,
    None,
}

/// A migration's identifier — `YYYYMMDD_HHMMSS_<snake_case_name>`.
///
/// Lexicographic ordering matches chronological ordering since the timestamp
/// is fixed-width.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(String);

impl Version {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn timestamp(&self) -> &str {
        &self.0[..15]
    }

    pub fn name(&self) -> &str {
        &self.0[16..]
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 17 {
            return Err(Error::InvalidName {
                name: s.to_string(),
            });
        }
        let date = &s[0..8];
        let sep1 = &s[8..9];
        let time = &s[9..15];
        let sep2 = &s[15..16];
        if !date.chars().all(|c| c.is_ascii_digit())
            || sep1 != "_"
            || !time.chars().all(|c| c.is_ascii_digit())
            || sep2 != "_"
        {
            return Err(Error::InvalidName {
                name: s.to_string(),
            });
        }
        let name = &s[16..];
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(Error::InvalidName {
                name: s.to_string(),
            });
        }
        Ok(Self(s.to_string()))
    }
}

/// A single discovered migration: identifier, SQL text, and tx mode.
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: Version,
    pub sql: String,
    pub tx_mode: TxMode,
}
