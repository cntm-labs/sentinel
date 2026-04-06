use crate::core::expr::Expr;
use crate::core::types::Value;

/// Controls what UPDATE returns.
#[derive(Debug, Clone)]
enum Returning {
    All,
    None,
}

/// Builder for UPDATE queries.
#[must_use = "query does nothing until .build() or .execute() is called"]
#[derive(Debug)]
pub struct UpdateQuery {
    table: String,
    sets: Vec<(String, Value)>,
    where_expr: Option<Expr>,
    returning: Returning,
}

impl UpdateQuery {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            sets: Vec::new(),
            where_expr: None,
            returning: Returning::All,
        }
    }

    pub fn set(mut self, column: &str, value: impl Into<Value>) -> Self {
        self.sets.push((column.to_owned(), value.into()));
        self
    }

    /// Simple WHERE id = $N filter.
    pub fn where_id(mut self, id: Value) -> Self {
        self.where_expr = Some(Expr::Compare {
            column: "\"id\"".to_owned(),
            op: "=",
            value: id,
        });
        self
    }

    /// Custom WHERE expression.
    pub fn where_(mut self, expr: Expr) -> Self {
        self.where_expr = Some(expr);
        self
    }

    pub fn no_returning(mut self) -> Self {
        self.returning = Returning::None;
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::new();
        let mut binds = Vec::new();
        let mut idx = 1usize;

        // UPDATE ... SET
        sql.push_str(&format!("UPDATE \"{}\" SET ", self.table));
        let set_clauses: Vec<String> = self
            .sets
            .iter()
            .map(|(col, val)| {
                let clause = format!("\"{}\" = ${}", col, idx);
                idx += 1;
                binds.push(val.clone());
                clause
            })
            .collect();
        sql.push_str(&set_clauses.join(", "));

        // WHERE
        if let Some(expr) = &self.where_expr {
            sql.push_str(&format!(" WHERE {}", expr.to_sql(idx)));
            binds.extend(expr.binds());
        }

        // RETURNING
        if matches!(self.returning, Returning::All) {
            sql.push_str(" RETURNING *");
        }

        (sql, binds)
    }
}
