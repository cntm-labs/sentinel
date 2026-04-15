use crate::core::expr::Expr;
use crate::core::query::{DeleteQuery, SelectQuery};
use crate::core::types::Value;

/// Metadata for a single column in a model.
pub struct ModelColumn {
    pub name: &'static str,
    pub column_type: &'static str,
    pub nullable: bool,
    pub has_default: bool,
}

/// Core trait that all Sentinel models implement.
///
/// In Phase 2, `#[derive(Model)]` generates this automatically.
/// For Phase 1, models implement this manually for testing.
pub trait Model: Sized {
    /// The PostgreSQL table name.
    const TABLE: &'static str;

    /// The primary key column name (default: "id").
    const PRIMARY_KEY: &'static str;

    /// Returns column metadata for this model.
    fn columns() -> &'static [ModelColumn];

    /// Decode a driver Row into this model instance.
    fn from_row(row: &driver::Row) -> driver::Result<Self>;

    /// Extract the primary key value from this model instance.
    fn primary_key_value(&self) -> Value;

    /// Start a SELECT query for this model's table.
    fn find() -> SelectQuery {
        SelectQuery::new(Self::TABLE)
    }

    /// SELECT ... WHERE id = $1
    fn find_by_id(id: Value) -> SelectQuery {
        SelectQuery::new(Self::TABLE).where_(Expr::Compare {
            column: format!("\"{}\"", Self::PRIMARY_KEY),
            op: "=",
            value: id,
        })
    }

    /// DELETE ... WHERE id = $1
    fn delete(id: Value) -> DeleteQuery {
        DeleteQuery::new(Self::TABLE).where_id(id)
    }
}
