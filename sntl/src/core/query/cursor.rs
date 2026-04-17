use crate::core::expr::{Expr, OrderExpr};
use crate::core::query::SelectQuery;
use crate::core::types::Value;

/// Cursor-based query builder for incremental fetching via Portal.
///
/// Usage:
/// ```ignore
/// let cursor = User::Find().Cursor();
/// let mut portal = cursor.Bind(&mut conn).await?;
/// let batch = conn.query_portal(&portal, 100).await?;
/// conn.close_portal(portal).await?;
/// ```
#[must_use = "cursor does nothing until .Bind() is called"]
pub struct CursorQuery {
    inner: SelectQuery,
}

impl CursorQuery {
    pub fn from_table(table: &str) -> Self {
        Self {
            inner: SelectQuery::new(table),
        }
    }

    pub fn from_select(select: SelectQuery) -> Self {
        Self { inner: select }
    }

    #[allow(non_snake_case)]
    pub fn Where(mut self, expr: Expr) -> Self {
        self.inner = self.inner.where_(expr);
        self
    }

    #[allow(non_snake_case)]
    pub fn OrderBy(mut self, order: OrderExpr) -> Self {
        self.inner = self.inner.order_by(order);
        self
    }

    #[allow(non_snake_case)]
    pub fn Build(&self) -> (String, Vec<Value>) {
        self.inner.build()
    }

    /// Bind a server-side portal for incremental fetching.
    ///
    /// Portal operations (`query_portal`, `close_portal`) require a direct
    /// `Connection` — they are not part of the `GenericClient` trait.
    #[allow(non_snake_case)]
    pub async fn Bind(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<driver::Portal> {
        let (sql, binds) = self.inner.build();
        let params: Vec<&(dyn driver::ToSql + Sync)> = binds
            .iter()
            .map(|v| v as &(dyn driver::ToSql + Sync))
            .collect();
        Ok(conn.bind_portal(&sql, &params).await?)
    }
}
