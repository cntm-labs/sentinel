use crate::core::types::Value;

/// Controls what INSERT returns.
#[derive(Debug, Clone)]
enum Returning {
    /// RETURNING * (default)
    All,
    /// RETURNING "col1", "col2"
    Columns(Vec<String>),
    /// No RETURNING clause
    None,
}

/// Builder for INSERT queries.
#[must_use = "query does nothing until .build() or .execute() is called"]
#[derive(Debug)]
pub struct InsertQuery {
    table: String,
    columns: Vec<String>,
    values: Vec<Value>,
    returning: Returning,
    on_conflict: Option<String>,
}

impl InsertQuery {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: Vec::new(),
            values: Vec::new(),
            returning: Returning::All,
            on_conflict: None,
        }
    }

    pub fn column(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.columns.push(name.to_owned());
        self.values.push(value.into());
        self
    }

    pub fn returning(mut self, cols: Vec<&str>) -> Self {
        self.returning = Returning::Columns(cols.into_iter().map(String::from).collect());
        self
    }

    pub fn no_returning(mut self) -> Self {
        self.returning = Returning::None;
        self
    }

    pub fn on_conflict_do_nothing(mut self, conflict_column: &str) -> Self {
        self.on_conflict = Some(conflict_column.to_owned());
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::new();

        // INSERT INTO
        let cols: Vec<String> = self.columns.iter().map(|c| format!("\"{c}\"")).collect();
        let placeholders: Vec<String> = (1..=self.values.len()).map(|i| format!("${i}")).collect();

        sql.push_str(&format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            self.table,
            cols.join(", "),
            placeholders.join(", ")
        ));

        // ON CONFLICT
        if let Some(col) = &self.on_conflict {
            sql.push_str(&format!(" ON CONFLICT (\"{col}\") DO NOTHING"));
        }

        // RETURNING
        match &self.returning {
            Returning::All => sql.push_str(" RETURNING *"),
            Returning::Columns(cols) => {
                let quoted: Vec<String> = cols.iter().map(|c| format!("\"{c}\"")).collect();
                sql.push_str(&format!(" RETURNING {}", quoted.join(", ")));
            }
            Returning::None => {}
        }

        (sql, self.values.clone())
    }
}
