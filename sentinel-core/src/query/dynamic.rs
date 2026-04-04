use crate::types::Value;

/// Dynamic query builder (Layer 4) — for queries built at runtime.
///
/// Always parameterized — values are never interpolated into SQL strings.
/// This is the escape hatch for queries that can't be expressed with the
/// typed builders, while still preventing SQL injection.
#[derive(Debug)]
pub struct QueryBuilder {
    table: String,
    columns: Vec<String>,
    wheres: Vec<(String, Value)>,
    order_bys: Vec<String>,
    limit: Option<u64>,
}

impl QueryBuilder {
    /// Start building a SELECT query.
    pub fn select_from(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: Vec::new(),
            wheres: Vec::new(),
            order_bys: Vec::new(),
            limit: None,
        }
    }

    /// Add a column to the SELECT clause.
    pub fn column(&mut self, name: &str) -> &mut Self {
        self.columns.push(name.to_owned());
        self
    }

    /// Add a WHERE column = $N condition.
    pub fn where_eq(&mut self, column: &str, value: impl Into<Value>) -> &mut Self {
        self.wheres.push((column.to_owned(), value.into()));
        self
    }

    /// Add ORDER BY column DESC.
    pub fn order_by_desc(&mut self, column: &str) -> &mut Self {
        self.order_bys.push(format!("\"{}\" DESC", column));
        self
    }

    /// Add ORDER BY column ASC.
    pub fn order_by_asc(&mut self, column: &str) -> &mut Self {
        self.order_bys.push(format!("\"{}\" ASC", column));
        self
    }

    /// Set LIMIT.
    pub fn limit(&mut self, n: u64) -> &mut Self {
        self.limit = Some(n);
        self
    }

    /// Build the final SQL and bind parameters.
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::new();
        let mut binds = Vec::new();

        // SELECT
        let cols = if self.columns.is_empty() {
            "*".to_owned()
        } else {
            self.columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ")
        };
        sql.push_str(&format!("SELECT {} FROM \"{}\"", cols, self.table));

        // WHERE
        if !self.wheres.is_empty() {
            let clauses: Vec<String> = self
                .wheres
                .iter()
                .enumerate()
                .map(|(i, (col, val))| {
                    binds.push(val.clone());
                    format!("\"{}\" = ${}", col, i + 1)
                })
                .collect();
            sql.push_str(&format!(" WHERE {}", clauses.join(" AND ")));
        }

        // ORDER BY
        if !self.order_bys.is_empty() {
            sql.push_str(&format!(" ORDER BY {}", self.order_bys.join(", ")));
        }

        // LIMIT
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        (sql, binds)
    }
}
