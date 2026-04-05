use crate::expr::Expr;
use crate::types::Value;

/// Builder for DELETE queries.
#[must_use = "query does nothing until .build() or .execute() is called"]
#[derive(Debug)]
pub struct DeleteQuery {
    table: String,
    where_expr: Option<Expr>,
    returning: bool,
}

impl DeleteQuery {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            where_expr: None,
            returning: false,
        }
    }

    pub fn where_id(mut self, id: Value) -> Self {
        self.where_expr = Some(Expr::Compare {
            column: "\"id\"".to_owned(),
            op: "=",
            value: id,
        });
        self
    }

    pub fn where_(mut self, expr: Expr) -> Self {
        self.where_expr = Some(expr);
        self
    }

    pub fn returning(mut self) -> Self {
        self.returning = true;
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = format!("DELETE FROM \"{}\"", self.table);
        let mut binds = Vec::new();

        if let Some(expr) = &self.where_expr {
            sql.push_str(&format!(" WHERE {}", expr.to_sql(1)));
            binds.extend(expr.binds());
        }

        if self.returning {
            sql.push_str(" RETURNING *");
        }

        (sql, binds)
    }

    /// Execute this DELETE and return the number of rows affected.
    pub async fn execute(
        self,
        conn: &mut sentinel_driver::Connection,
    ) -> crate::error::Result<u64> {
        let (sql, binds) = self.build();
        let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> =
            binds.iter().map(|v| v as &(dyn sentinel_driver::ToSql + Sync)).collect();
        Ok(conn.execute(&sql, &params).await?)
    }
}
