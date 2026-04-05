//! Async execution methods for query builders.
//!
//! These methods require a live database connection and are excluded from
//! unit-test coverage. They will be covered by integration tests.

use super::{DeleteQuery, InsertQuery, SelectQuery};

/// Helper: convert `Vec<Value>` binds into driver params slice.
fn to_params(binds: &[crate::types::Value]) -> Vec<&(dyn sentinel_driver::ToSql + Sync)> {
    binds
        .iter()
        .map(|v| v as &(dyn sentinel_driver::ToSql + Sync))
        .collect()
}

impl SelectQuery {
    /// Execute this query and fetch all rows.
    pub async fn fetch_all(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<Vec<sentinel_driver::Row>> {
        let (sql, binds) = self.build();
        Ok(conn.query(&sql, &to_params(&binds)).await?)
    }

    /// Execute this query and fetch exactly one row.
    ///
    /// Returns `Error::NotFound` if no rows are returned.
    pub async fn fetch_one(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<sentinel_driver::Row> {
        let (sql, binds) = self.build();
        conn.query_one(&sql, &to_params(&binds))
            .await
            .map_err(Into::into)
    }

    /// Execute this query and fetch an optional row.
    pub async fn fetch_optional(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<Option<sentinel_driver::Row>> {
        let (sql, binds) = self.build();
        Ok(conn.query_opt(&sql, &to_params(&binds)).await?)
    }
}

impl InsertQuery {
    /// Execute this INSERT and return all rows via RETURNING clause.
    pub async fn fetch_returning(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<Vec<sentinel_driver::Row>> {
        let (sql, binds) = self.build();
        Ok(conn.query(&sql, &to_params(&binds)).await?)
    }

    /// Execute this INSERT and return the number of rows affected.
    pub async fn execute(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<u64> {
        let (sql, binds) = self.build();
        Ok(conn.execute(&sql, &to_params(&binds)).await?)
    }
}

impl DeleteQuery {
    /// Execute this DELETE and return the number of rows affected.
    pub async fn execute(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<u64> {
        let (sql, binds) = self.build();
        Ok(conn.execute(&sql, &to_params(&binds)).await?)
    }
}
