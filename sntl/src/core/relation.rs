//! Relation type descriptors for compile-time N+1 prevention.
//!
//! Defines `HasMany<T>`, `HasOne<T>`, `BelongsTo<T>` descriptors
//! and `Loaded`/`Unloaded` state markers for type-state relations.

use std::marker::PhantomData;

/// Marker: relation data has been loaded from the database.
pub struct Loaded;

/// Marker: relation data has NOT been loaded (default state).
pub struct Unloaded;

/// Relation cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    HasMany,
    HasOne,
    BelongsTo,
}

/// One-to-many relation descriptor.
pub struct HasMany<T> {
    fk: &'static str,
    _target: PhantomData<T>,
}

impl<T> HasMany<T> {
    pub const fn new(foreign_key: &'static str) -> Self {
        Self {
            fk: foreign_key,
            _target: PhantomData,
        }
    }
    pub fn foreign_key(&self) -> &'static str {
        self.fk
    }
    pub fn kind(&self) -> RelationKind {
        RelationKind::HasMany
    }
}

/// One-to-one relation descriptor.
pub struct HasOne<T> {
    fk: &'static str,
    _target: PhantomData<T>,
}

impl<T> HasOne<T> {
    pub const fn new(foreign_key: &'static str) -> Self {
        Self {
            fk: foreign_key,
            _target: PhantomData,
        }
    }
    pub fn foreign_key(&self) -> &'static str {
        self.fk
    }
    pub fn kind(&self) -> RelationKind {
        RelationKind::HasOne
    }
}

/// Inverse relation descriptor (many-to-one).
pub struct BelongsTo<T> {
    fk: &'static str,
    _target: PhantomData<T>,
}

impl<T> BelongsTo<T> {
    pub const fn new(foreign_key: &'static str) -> Self {
        Self {
            fk: foreign_key,
            _target: PhantomData,
        }
    }
    pub fn foreign_key(&self) -> &'static str {
        self.fk
    }
    pub fn kind(&self) -> RelationKind {
        RelationKind::BelongsTo
    }
}

// ---------------------------------------------------------------------------
// RelationSpec — runtime metadata for batch-loading with Filter/OrderBy/Limit
// ---------------------------------------------------------------------------

use crate::core::expr::{Expr, OrderExpr};
use crate::core::types::Value;

/// Runtime metadata for a relation include — carries filter, order, limit.
///
/// Created as a `const` by macro-generated code, then refined at runtime
/// via `.Filter()`, `.OrderBy()`, `.Limit()` builder methods.
#[derive(Debug)]
pub struct RelationSpec {
    name: &'static str,
    foreign_key: &'static str,
    target_table: &'static str,
    kind: RelationKind,
    filters: Vec<Expr>,
    order_bys: Vec<OrderExpr>,
    limit: Option<u64>,
}

impl RelationSpec {
    pub fn new(
        name: &'static str,
        foreign_key: &'static str,
        target_table: &'static str,
        kind: RelationKind,
    ) -> Self {
        Self {
            name,
            foreign_key,
            target_table,
            kind,
            filters: Vec::new(),
            order_bys: Vec::new(),
            limit: None,
        }
    }

    /// Const-compatible constructor for macro-generated associated constants.
    pub const fn new_const(
        name: &'static str,
        foreign_key: &'static str,
        target_table: &'static str,
        kind: RelationKind,
    ) -> Self {
        Self {
            name,
            foreign_key,
            target_table,
            kind,
            filters: Vec::new(),
            order_bys: Vec::new(),
            limit: None,
        }
    }

    pub fn name(&self) -> &str {
        self.name
    }
    pub fn foreign_key(&self) -> &str {
        self.foreign_key
    }
    pub fn target_table(&self) -> &str {
        self.target_table
    }
    pub fn kind(&self) -> RelationKind {
        self.kind
    }
    pub fn limit(&self) -> Option<u64> {
        self.limit
    }
    pub fn has_filters(&self) -> bool {
        !self.filters.is_empty()
    }

    #[allow(non_snake_case)]
    pub fn Filter(mut self, expr: Expr) -> Self {
        self.filters.push(expr);
        self
    }

    #[allow(non_snake_case)]
    pub fn OrderBy(mut self, order: OrderExpr) -> Self {
        self.order_bys.push(order);
        self
    }

    #[allow(non_snake_case)]
    pub fn Limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    /// Build a batch-loading SQL: `SELECT * FROM target WHERE fk IN ($1, $2, ...)`
    pub fn build_batch_sql(&self, parent_ids: &[Value]) -> (String, Vec<Value>) {
        let mut sql = format!(
            "SELECT \"{}\".* FROM \"{}\"",
            self.target_table, self.target_table
        );
        let mut binds = Vec::new();
        let mut idx = 1usize;

        // WHERE fk IN (...)
        let placeholders: Vec<String> = parent_ids
            .iter()
            .map(|v| {
                binds.push(v.clone());
                let p = format!("${idx}");
                idx += 1;
                p
            })
            .collect();
        sql.push_str(&format!(
            " WHERE \"{}\" IN ({})",
            self.foreign_key,
            placeholders.join(", ")
        ));

        // Additional filters (AND ...)
        for filter in &self.filters {
            sql.push_str(&format!(" AND {}", filter.to_sql(idx)));
            binds.extend(filter.binds());
            idx += filter.bind_count();
        }

        // ORDER BY
        if !self.order_bys.is_empty() {
            let orders: Vec<String> = self.order_bys.iter().map(|o| o.to_sql_bare()).collect();
            sql.push_str(&format!(" ORDER BY {}", orders.join(", ")));
        }

        // LIMIT
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        (sql, binds)
    }
}
