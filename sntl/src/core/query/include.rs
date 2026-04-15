use std::collections::HashMap;
use std::marker::PhantomData;

use crate::core::expr::{Expr, OrderExpr};
use crate::core::model::Model;
use crate::core::query::SelectQuery;
use crate::core::relation::{
    IncludeTransition, RelationInclude, RelationSpec, RelationStore, WithRelations,
};
use crate::core::types::Value;

/// Query builder that tracks included relations in the type system.
///
/// Each `.Include()` call transitions the State type parameter,
/// ensuring compile-time safety for relation access on the result.
#[must_use = "query does nothing until .FetchAll() or .Build() is called"]
pub struct IncludeQuery<M, State = ()> {
    inner: SelectQuery,
    includes: Vec<RelationSpec>,
    _marker: PhantomData<(M, State)>,
}

impl<M, S> IncludeQuery<M, S> {
    pub fn from_table(table: &str) -> Self {
        Self {
            inner: SelectQuery::new(table),
            includes: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn from_parts(select: SelectQuery, includes: Vec<RelationSpec>) -> Self {
        Self {
            inner: select,
            includes,
            _marker: PhantomData,
        }
    }

    /// Add a relation include with compile-time state transition.
    pub fn include_rel<Rel>(
        self,
        spec: RelationSpec,
    ) -> IncludeQuery<M, <() as IncludeTransition<M, S, Rel>>::Next>
    where
        (): IncludeTransition<M, S, Rel>,
    {
        let mut includes = self.includes;
        includes.push(spec);
        IncludeQuery {
            inner: self.inner,
            includes,
            _marker: PhantomData,
        }
    }

    /// Type-safe Include using RelationInclude marker.
    #[allow(non_snake_case)]
    pub fn Include<Rel>(
        self,
        inc: RelationInclude<M, Rel>,
    ) -> IncludeQuery<M, <() as IncludeTransition<M, S, Rel>>::Next>
    where
        (): IncludeTransition<M, S, Rel>,
    {
        self.include_rel::<Rel>(inc.into_spec())
    }

    #[allow(non_snake_case)]
    pub fn Where(mut self, expr: Expr) -> Self {
        self.inner = self.inner.where_(expr);
        self
    }

    #[allow(non_snake_case)]
    pub fn OrderBy(mut self, order: OrderExpr) -> Self {
        self.inner = self.inner.order_by(order);
        self
    }

    #[allow(non_snake_case)]
    pub fn Limit(mut self, n: u64) -> Self {
        self.inner = self.inner.limit(n);
        self
    }

    #[allow(non_snake_case)]
    pub fn Build(&self) -> (String, Vec<Value>) {
        self.inner.build()
    }

    pub fn included_specs(&self) -> &[RelationSpec] {
        &self.includes
    }

    pub fn into_parts(self) -> (SelectQuery, Vec<RelationSpec>) {
        (self.inner, self.includes)
    }
}

// ---------------------------------------------------------------------------
// Async execution — FetchOne / FetchAll
// ---------------------------------------------------------------------------

/// Helper: convert `Vec<Value>` binds into driver params slice.
fn to_params(binds: &[Value]) -> Vec<&(dyn driver::ToSql + Sync)> {
    binds
        .iter()
        .map(|v| v as &(dyn driver::ToSql + Sync))
        .collect()
}

impl<M: Model, S> IncludeQuery<M, S> {
    /// Execute main query + batch load all included relations, returning one result.
    #[allow(non_snake_case)]
    pub async fn FetchOne(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<WithRelations<M, S>> {
        let (select, includes) = self.into_parts();
        let row = select.fetch_one(conn).await?;
        let model = M::from_row(&row).map_err(crate::core::Error::from)?;
        let pk = model.primary_key_value();

        let mut store = RelationStore::new();
        for spec in &includes {
            let (sql, binds) = spec.build_batch_sql(std::slice::from_ref(&pk));
            let rows = conn.query(&sql, &to_params(&binds)).await?;
            store.insert_decoded(spec.name(), rows);
        }

        Ok(WithRelations::new(model, store))
    }

    /// Execute main query + batch load all included relations, returning all results.
    ///
    /// For N models with K includes, this executes exactly K+1 queries total.
    /// Relation rows are stored as shared `Vec<Row>` — macro-generated accessors
    /// filter by FK at access time using the model's concrete PK type.
    #[allow(non_snake_case)]
    pub async fn FetchAll(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<Vec<WithRelations<M, S>>> {
        let (select, includes) = self.into_parts();
        let main_rows = select.fetch_all(conn).await?;
        let models: Vec<M> = main_rows
            .iter()
            .map(|r| M::from_row(r).map_err(crate::core::Error::from))
            .collect::<Result<_, _>>()?;

        if includes.is_empty() {
            return Ok(models
                .into_iter()
                .map(|m| WithRelations::new(m, RelationStore::new()))
                .collect());
        }

        let pks: Vec<Value> = models.iter().map(|m| m.primary_key_value()).collect();

        // One batch query per relation — shared across all models
        let mut relation_data: HashMap<&str, std::sync::Arc<Vec<driver::Row>>> = HashMap::new();
        for spec in &includes {
            let (sql, binds) = spec.build_batch_sql(&pks);
            let rows = conn.query(&sql, &to_params(&binds)).await?;
            relation_data.insert(spec.name(), std::sync::Arc::new(rows));
        }

        // Each model gets a store with shared references to all relation rows.
        // Macro-generated accessors will filter by FK using the concrete PK type.
        let results: Vec<WithRelations<M, S>> = models
            .into_iter()
            .enumerate()
            .map(|(i, model)| {
                let mut store = RelationStore::new();
                let pk = pks[i].clone();

                for spec in &includes {
                    if let Some(rows) = relation_data.get(spec.name()) {
                        store.insert_decoded(
                            spec.name(),
                            crate::core::relation::RelationRows {
                                rows: std::sync::Arc::clone(rows),
                                parent_pk: pk.clone(),
                                foreign_key: spec.foreign_key(),
                            },
                        );
                    }
                }

                WithRelations::new(model, store)
            })
            .collect();

        Ok(results)
    }
}
