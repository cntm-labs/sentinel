//! Relation type descriptors for compile-time N+1 prevention.
//!
//! Defines `HasMany<T>`, `HasOne<T>`, `BelongsTo<T>` descriptors
//! and `Loaded`/`Unloaded` state markers for type-state relations.

use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;

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

    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn foreign_key(&self) -> &'static str {
        self.foreign_key
    }
    pub fn target_table(&self) -> &'static str {
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

// ---------------------------------------------------------------------------
// RelationStore — type-erased storage for loaded relation data
// ---------------------------------------------------------------------------

/// Type-erased storage for loaded relation data.
///
/// Keyed by relation name. Stores pre-decoded Rust values as `Box<dyn Any>`.
/// Decode happens at Include execution time, not at accessor time.
pub struct RelationStore {
    data: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
}

impl RelationStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Store pre-decoded relation data.
    pub fn insert_decoded<T: Any + Send + Sync>(&mut self, name: &'static str, data: T) {
        self.data.insert(name, Box::new(data));
    }

    /// Retrieve typed relation data by name.
    pub fn get<T: Any>(&self, name: &str) -> Option<&T> {
        self.data.get(name)?.downcast_ref::<T>()
    }
}

impl Default for RelationStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// WithRelations — model wrapper with type-state relation tracking
// ---------------------------------------------------------------------------

/// Wrapper that pairs a model with its loaded relation data.
///
/// `M` = model type, `State` = tuple of Loaded/Unloaded per relation.
/// Deref to `M` for transparent field access.
pub struct WithRelations<M, State = ()> {
    model: M,
    relations: RelationStore,
    _state: PhantomData<State>,
}

impl<M, S> WithRelations<M, S> {
    pub fn new(model: M, relations: RelationStore) -> Self {
        Self {
            model,
            relations,
            _state: PhantomData,
        }
    }

    pub fn into_inner(self) -> M {
        self.model
    }

    pub fn relations(&self) -> &RelationStore {
        &self.relations
    }
}

impl<M> WithRelations<M, ()> {
    /// Create a bare WithRelations with no loaded relations.
    pub fn bare(model: M) -> Self {
        Self::new(model, RelationStore::new())
    }
}

// ---------------------------------------------------------------------------
// RelationLoaded — trait gating access to loaded relation data
// ---------------------------------------------------------------------------

/// Trait gating access to relation data. Only implemented when the relation
/// is in `Loaded` state. Attempting to access an unloaded relation produces
/// a compile error with a helpful diagnostic message.
#[diagnostic::on_unimplemented(
    message = "relation `{Rel}` was not included in the query",
    label = "call .Include() to load this relation before accessing it"
)]
pub trait RelationLoaded<Rel> {
    type Output: ?Sized;
    fn get_relation(&self) -> &Self::Output;
}

// ---------------------------------------------------------------------------
// ModelRelations — associates a model with its bare relation state
// ---------------------------------------------------------------------------

/// Associates a model with its default (all-unloaded) relation state.
///
/// Macro generates: `impl ModelRelations for User { type BareState = (Unloaded, Unloaded); }`
pub trait ModelRelations {
    type BareState;
}

// ---------------------------------------------------------------------------
// IncludeTransition — compile-time state transitions for Include()
// ---------------------------------------------------------------------------

/// Trait for compile-time state transitions when including a relation.
///
/// `M` = model, `Current` = current state tuple, `Rel` = relation marker.
/// Macro generates impls that flip the Rel's position from Unloaded → Loaded.
pub trait IncludeTransition<M, Current, Rel> {
    type Next;
}

// ---------------------------------------------------------------------------
// RelationInclude — typed relation include marker
// ---------------------------------------------------------------------------

/// Typed relation include marker — carries both compile-time type info
/// and runtime RelationSpec for query execution.
pub struct RelationInclude<M, Rel> {
    spec: RelationSpec,
    _marker: PhantomData<(M, Rel)>,
}

impl<M, Rel> RelationInclude<M, Rel> {
    pub fn new(spec: RelationSpec) -> Self {
        Self {
            spec,
            _marker: PhantomData,
        }
    }

    pub fn spec(&self) -> &RelationSpec {
        &self.spec
    }

    pub fn into_spec(self) -> RelationSpec {
        self.spec
    }
}

// ---------------------------------------------------------------------------
// RelationRows — batch-loaded relation row data for accessor decoding
// ---------------------------------------------------------------------------

/// Holds batch-loaded relation rows with parent PK for FK-based filtering.
///
/// Stored in `RelationStore` during `IncludeQuery::FetchAll()`.
/// Macro-generated accessors extract matching rows using the FK column.
pub struct RelationRows {
    /// All rows returned by the batch relation query (shared across parents).
    pub rows: std::sync::Arc<Vec<driver::Row>>,
    /// The parent model's primary key value for FK filtering.
    pub parent_pk: Value,
    /// The FK column name in the relation rows.
    pub foreign_key: &'static str,
}

impl<M, S> Deref for WithRelations<M, S> {
    type Target = M;
    fn deref(&self) -> &M {
        &self.model
    }
}
