use crate::cache::{CacheEntry, ColumnInfo, ParamInfo, QueryKind};
use crate::error::{Error, Result};
use crate::schema::Schema;

pub struct ResolveInput<'a> {
    pub sql: &'a str,
    pub cache_entry: &'a CacheEntry,
    pub schema: &'a Schema,
    pub overrides_nullable: &'a [String],
    pub overrides_non_null: &'a [String],
    /// Names of array columns whose elements the caller asserts are non-null.
    /// Used by codegen to emit `Vec<T>` instead of `Vec<Option<T>>`.
    pub overrides_non_null_elements: &'a [String],
    pub strict: bool,
}

#[derive(Debug)]
pub struct ResolvedQuery {
    pub params: Vec<ParamInfo>,
    pub columns: Vec<ColumnInfo>,
    pub query_kind: QueryKind,
    pub has_returning: bool,
    /// Forwarded from `ResolveInput.overrides_non_null_elements` so codegen
    /// can decide between `Vec<T>` and `Vec<Option<T>>` without re-receiving
    /// the input.
    pub non_null_elements: Vec<String>,
}

pub fn resolve_offline(input: ResolveInput<'_>) -> Result<ResolvedQuery> {
    let mut columns = input.cache_entry.columns.clone();

    // Validate overrides refer to real columns
    for name in input
        .overrides_nullable
        .iter()
        .chain(input.overrides_non_null.iter())
    {
        if !columns.iter().any(|c| &c.name == name) {
            return Err(Error::Config(format!(
                "override refers to unknown column `{name}`"
            )));
        }
    }

    // Validate element overrides reference real array columns
    for name in input.overrides_non_null_elements.iter() {
        let col = columns.iter().find(|c| &c.name == name).ok_or_else(|| {
            Error::Config(format!(
                "override `non_null_elements` references unknown column `{name}`"
            ))
        })?;
        if col.element_type.is_none() {
            return Err(Error::Config(format!(
                "override `non_null_elements` references `{name}` which is not an array column"
            )));
        }
    }

    for c in columns.iter_mut() {
        if input.overrides_nullable.iter().any(|n| n == &c.name) {
            c.nullable = true;
        }
        if input.overrides_non_null.iter().any(|n| n == &c.name) {
            c.nullable = false;
        }
    }

    // Sanity: every column origin, if set, must exist in schema. Tolerate missing
    // origins in non-strict mode (complex expressions) but warn in strict mode.
    if input.strict {
        for c in &columns {
            if let Some(origin) = &c.origin {
                if input
                    .schema
                    .find_column(&origin.table, &origin.column)
                    .is_none()
                {
                    return Err(Error::UnknownColumn {
                        table: origin.table.clone(),
                        column: origin.column.clone(),
                    });
                }
            }
        }
    }

    let _ = input.sql; // reserved for future cross-check against cache SQL
    Ok(ResolvedQuery {
        params: input.cache_entry.params.clone(),
        columns,
        query_kind: input.cache_entry.query_kind,
        has_returning: input.cache_entry.has_returning,
        non_null_elements: input.overrides_non_null_elements.to_vec(),
    })
}
