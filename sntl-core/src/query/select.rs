use crate::expr::{Expr, OrderExpr};
use crate::types::Value;

/// Builder for SELECT queries with parameterized bind values.
#[must_use = "query does nothing until .build() or .fetch_all() is called"]
#[derive(Debug)]
pub struct SelectQuery {
    table: String,
    columns: Option<Vec<String>>,
    wheres: Vec<Expr>,
    order_bys: Vec<OrderExpr>,
    limit: Option<u64>,
    offset: Option<u64>,
    for_update: bool,
}

impl SelectQuery {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: None,
            wheres: Vec::new(),
            order_bys: Vec::new(),
            limit: None,
            offset: None,
            for_update: false,
        }
    }

    pub fn columns(mut self, cols: Vec<&str>) -> Self {
        self.columns = Some(cols.into_iter().map(String::from).collect());
        self
    }

    pub fn where_(mut self, expr: Expr) -> Self {
        self.wheres.push(expr);
        self
    }

    pub fn order_by(mut self, order: OrderExpr) -> Self {
        self.order_bys.push(order);
        self
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: u64) -> Self {
        self.offset = Some(n);
        self
    }

    pub fn for_update(mut self) -> Self {
        self.for_update = true;
        self
    }

    /// Build the SQL string and bind parameters.
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::new();
        let mut binds = Vec::new();

        // SELECT clause
        sql.push_str("SELECT ");
        match &self.columns {
            Some(cols) => {
                let qualified: Vec<String> = cols
                    .iter()
                    .map(|c| format!("\"{}\".\"{c}\"", self.table))
                    .collect();
                sql.push_str(&qualified.join(", "));
            }
            None => {
                sql.push_str(&format!("\"{}\".*", self.table));
            }
        }

        // FROM clause
        sql.push_str(&format!(" FROM \"{}\"", self.table));

        // WHERE clause
        if !self.wheres.is_empty() {
            let combined = self.wheres.iter().cloned().reduce(|a, b| a.and(b)).unwrap();
            let bind_start = binds.len() + 1;
            sql.push_str(&format!(" WHERE {}", combined.to_sql(bind_start)));
            binds.extend(combined.binds());
        }

        // ORDER BY clause
        if !self.order_bys.is_empty() {
            let orders: Vec<String> = self.order_bys.iter().map(|o| o.to_sql_bare()).collect();
            sql.push_str(&format!(" ORDER BY {}", orders.join(", ")));
        }

        // LIMIT / OFFSET
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {offset}"));
        }

        // FOR UPDATE
        if self.for_update {
            sql.push_str(" FOR UPDATE");
        }

        (sql, binds)
    }
}
