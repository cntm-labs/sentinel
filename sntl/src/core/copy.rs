//! COPY protocol support for bulk operations.

use crate::core::model::Model;

/// Build a COPY IN SQL statement from Model metadata.
///
/// Returns: `COPY "table" ("col1", "col2", ...) FROM STDIN BINARY`
pub fn copy_in_sql<M: Model>() -> String {
    let col_list = M::columns()
        .iter()
        .map(|c| format!("\"{}\"", c.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!("COPY \"{}\" ({}) FROM STDIN BINARY", M::TABLE, col_list)
}
