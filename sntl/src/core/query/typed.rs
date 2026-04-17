use crate::core::query::SelectQuery;
use crate::core::types::Value;
use driver::ToSql;

/// A typed query that uses `query_typed()` to skip the Prepare step.
///
/// This is an optimization for queries where the parameter types are
/// known at build time, avoiding an extra round-trip to the server.
///
/// Created via `ModelQuery::Typed()` or `TypedQuery::from_select()`.
#[must_use = "query does nothing until .FetchAll() or .Build() is called"]
pub struct TypedQuery {
    inner: SelectQuery,
}

impl TypedQuery {
    pub fn from_select(select: SelectQuery) -> Self {
        Self { inner: select }
    }

    #[allow(non_snake_case)]
    pub fn Build(&self) -> (String, Vec<Value>) {
        self.inner.build()
    }

    /// Execute using `query_typed()` — skips Prepare for fewer round-trips.
    ///
    /// Requires a direct `Connection` since `query_typed` is not part of `GenericClient`.
    #[allow(non_snake_case)]
    pub async fn FetchAll(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<Vec<driver::Row>> {
        let (sql, binds) = self.inner.build();
        let typed_params: Vec<(&(dyn driver::ToSql + Sync), driver::Oid)> = binds
            .iter()
            .map(|v| (v as &(dyn driver::ToSql + Sync), v.oid()))
            .collect();
        Ok(conn.query_typed(&sql, &typed_params).await?)
    }

    /// Execute using `query_typed_one()` — single row, skip Prepare.
    #[allow(non_snake_case)]
    pub async fn FetchOne(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<driver::Row> {
        let (sql, binds) = self.inner.build();
        let typed_params: Vec<(&(dyn driver::ToSql + Sync), driver::Oid)> = binds
            .iter()
            .map(|v| (v as &(dyn driver::ToSql + Sync), v.oid()))
            .collect();
        conn.query_typed_one(&sql, &typed_params)
            .await
            .map_err(Into::into)
    }

    /// Execute using `query_typed_opt()` — optional row, skip Prepare.
    #[allow(non_snake_case)]
    pub async fn FetchOptional(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<Option<driver::Row>> {
        let (sql, binds) = self.inner.build();
        let typed_params: Vec<(&(dyn driver::ToSql + Sync), driver::Oid)> = binds
            .iter()
            .map(|v| (v as &(dyn driver::ToSql + Sync), v.oid()))
            .collect();
        Ok(conn.query_typed_opt(&sql, &typed_params).await?)
    }
}
