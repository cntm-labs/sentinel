# Phase 4: Type-State Relations Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Compile-time N+1 prevention — accessing an unloaded relation is a compile error via per-relation generic parameters on model structs.

**Architecture:** `#[sentinel(relations)]` attribute macro parses relation declarations on impl blocks, generates type-state generics, gated accessor methods, relation constants, and WHERE IN batch-loading SQL. PascalCase query API wraps existing query builders.

**Tech Stack:** Rust proc macros (syn, quote, darling), PhantomData generics, `#[diagnostic::on_unimplemented]` (Rust 1.78+), sentinel-driver for integration tests.

---

### Task 1: Core Relation Types (sntl crate)

**Files:**
- Create: `sntl/src/core/relation.rs`
- Modify: `sntl/src/core/mod.rs`

**Step 1: Write failing test**

Create `sntl/tests/relation_types_test.rs`:

```rust
use sntl::core::relation::{HasMany, HasOne, BelongsTo, RelationKind};

#[test]
fn has_many_stores_foreign_key() {
    let rel = HasMany::<String>::new("user_id");
    assert_eq!(rel.foreign_key(), "user_id");
    assert_eq!(rel.kind(), RelationKind::HasMany);
}

#[test]
fn has_one_stores_foreign_key() {
    let rel = HasOne::<String>::new("user_id");
    assert_eq!(rel.foreign_key(), "user_id");
    assert_eq!(rel.kind(), RelationKind::HasOne);
}

#[test]
fn belongs_to_stores_foreign_key() {
    let rel = BelongsTo::<String>::new("user_id");
    assert_eq!(rel.foreign_key(), "user_id");
    assert_eq!(rel.kind(), RelationKind::BelongsTo);
}
```

**Step 2:** Run `cargo test -p sntl --test relation_types_test` → FAIL (module doesn't exist)

**Step 3: Implement**

Create `sntl/src/core/relation.rs`:

```rust
use std::marker::PhantomData;

/// Loaded/Unloaded state markers for type-state relations.
pub struct Loaded;
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
        Self { fk: foreign_key, _target: PhantomData }
    }
    pub fn foreign_key(&self) -> &'static str { self.fk }
    pub fn kind(&self) -> RelationKind { RelationKind::HasMany }
}

/// One-to-one relation descriptor.
pub struct HasOne<T> {
    fk: &'static str,
    _target: PhantomData<T>,
}

impl<T> HasOne<T> {
    pub const fn new(foreign_key: &'static str) -> Self {
        Self { fk: foreign_key, _target: PhantomData }
    }
    pub fn foreign_key(&self) -> &'static str { self.fk }
    pub fn kind(&self) -> RelationKind { RelationKind::HasOne }
}

/// Inverse relation descriptor (many-to-one).
pub struct BelongsTo<T> {
    fk: &'static str,
    _target: PhantomData<T>,
}

impl<T> BelongsTo<T> {
    pub const fn new(foreign_key: &'static str) -> Self {
        Self { fk: foreign_key, _target: PhantomData }
    }
    pub fn foreign_key(&self) -> &'static str { self.fk }
    pub fn kind(&self) -> RelationKind { RelationKind::BelongsTo }
}
```

Add to `sntl/src/core/mod.rs`:
```rust
pub mod relation;
```

Add to prelude: `pub use crate::core::relation::{HasMany, HasOne, BelongsTo, Loaded, Unloaded};`

**Step 4:** Run `cargo test -p sntl --test relation_types_test` → PASS

**Step 5:** Commit `feat(core): add relation type descriptors — HasMany, HasOne, BelongsTo`

---

### Task 2: RelationSpec for Include Metadata

**Files:**
- Modify: `sntl/src/core/relation.rs`
- Create: `sntl/tests/relation_spec_test.rs`

**Step 1: Write failing test**

```rust
use sntl::core::relation::{RelationSpec, RelationKind};
use sntl::core::expr::Expr;

#[test]
fn relation_spec_basic() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany);
    assert_eq!(spec.name(), "posts");
    assert_eq!(spec.foreign_key(), "user_id");
    assert_eq!(spec.target_table(), "posts");
}

#[test]
fn relation_spec_with_filter() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany)
        .Filter(sntl::core::Column::new("posts", "published").eq(true))
        .Limit(5);
    assert_eq!(spec.limit(), Some(5));
    assert!(spec.has_filters());
}

#[test]
fn relation_spec_generates_where_in_sql() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany);
    let (sql, _) = spec.build_batch_sql(&[1i32.into(), 2i32.into()]);
    assert_eq!(sql, "SELECT \"posts\".* FROM \"posts\" WHERE \"user_id\" IN ($1, $2)");
}

#[test]
fn relation_spec_with_filter_generates_sql() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany)
        .Filter(sntl::core::Column::new("posts", "published").eq(true))
        .OrderBy(sntl::core::Column::new("posts", "created_at").desc())
        .Limit(5);
    let (sql, _) = spec.build_batch_sql(&[1i32.into()]);
    assert!(sql.contains("WHERE \"user_id\" IN ($1)"));
    assert!(sql.contains("AND \"posts\".\"published\" = $2"));
    assert!(sql.contains("ORDER BY"));
    assert!(sql.contains("LIMIT 5"));
}
```

**Step 2:** Run test → FAIL

**Step 3: Implement**

Add `RelationSpec` to `sntl/src/core/relation.rs`:

```rust
use crate::core::expr::{Expr, OrderExpr};
use crate::core::types::Value;

/// Runtime metadata for a relation include — carries filter, order, limit.
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
    pub fn new(name: &'static str, foreign_key: &'static str, target_table: &'static str, kind: RelationKind) -> Self {
        Self { name, foreign_key, target_table, kind, filters: Vec::new(), order_bys: Vec::new(), limit: None }
    }

    pub fn name(&self) -> &str { self.name }
    pub fn foreign_key(&self) -> &str { self.foreign_key }
    pub fn target_table(&self) -> &str { self.target_table }
    pub fn kind(&self) -> RelationKind { self.kind }
    pub fn limit(&self) -> Option<u64> { self.limit }
    pub fn has_filters(&self) -> bool { !self.filters.is_empty() }

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

    /// Build a batch-loading SQL: SELECT * FROM target WHERE fk IN ($1, $2, ...)
    pub fn build_batch_sql(&self, parent_ids: &[Value]) -> (String, Vec<Value>) {
        let mut sql = format!("SELECT \"{}\".* FROM \"{}\"", self.target_table, self.target_table);
        let mut binds = Vec::new();
        let mut idx = 1usize;

        // WHERE fk IN (...)
        let placeholders: Vec<String> = parent_ids.iter().map(|v| {
            binds.push(v.clone());
            let p = format!("${idx}");
            idx += 1;
            p
        }).collect();
        sql.push_str(&format!(" WHERE \"{}\" IN ({})", self.foreign_key, placeholders.join(", ")));

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
```

**Step 4:** Run test → PASS

**Step 5:** Commit `feat(core): add RelationSpec with Filter/OrderBy/Limit and batch SQL generation`

---

### Task 3: PascalCase Query Wrapper

**Files:**
- Create: `sntl/src/core/query/pascal.rs`
- Modify: `sntl/src/core/query/mod.rs`
- Create: `sntl/tests/pascal_query_test.rs`

**Step 1: Write failing test**

```rust
use sntl::core::query::ModelQuery;
use sntl::core::{Column, Value};

#[test]
fn pascal_find_builds_select() {
    let q = ModelQuery::from_table("users");
    let (sql, _) = q.Build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
}

#[test]
fn pascal_where_and_order() {
    let col = Column::new("users", "email");
    let q = ModelQuery::from_table("users")
        .Where(col.eq("test@test.com"))
        .OrderBy(Column::new("users", "name").asc())
        .Limit(10);
    let (sql, binds) = q.Build();
    assert!(sql.contains("WHERE"));
    assert!(sql.contains("ORDER BY"));
    assert!(sql.contains("LIMIT 10"));
    assert_eq!(binds.len(), 1);
}
```

**Step 2:** Run test → FAIL

**Step 3: Implement**

Create `sntl/src/core/query/pascal.rs` — a thin PascalCase wrapper around `SelectQuery`:

```rust
use crate::core::expr::{Expr, OrderExpr};
use crate::core::query::SelectQuery;
use crate::core::types::Value;

/// PascalCase query builder wrapping SelectQuery.
/// Generated by derive(Model) for each model's Find()/FindId() methods.
#[must_use = "query does nothing until .FetchAll() or .Build() is called"]
pub struct ModelQuery {
    inner: SelectQuery,
}

impl ModelQuery {
    pub fn from_table(table: &str) -> Self {
        Self { inner: SelectQuery::new(table) }
    }

    pub fn from_select(select: SelectQuery) -> Self {
        Self { inner: select }
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
    pub fn Offset(mut self, n: u64) -> Self {
        self.inner = self.inner.offset(n);
        self
    }

    #[allow(non_snake_case)]
    pub fn Build(&self) -> (String, Vec<Value>) {
        self.inner.build()
    }

    /// Access the inner SelectQuery for execution.
    pub fn into_inner(self) -> SelectQuery {
        self.inner
    }
}
```

Add `pub mod pascal;` to `sntl/src/core/query/mod.rs` and `pub use pascal::ModelQuery;`

**Step 4:** Run test → PASS

**Step 5:** Commit `feat(core): add PascalCase ModelQuery wrapper`

---

### Task 4: Relation IR in sntl-macros

**Files:**
- Create: `sntl-macros/src/relation/mod.rs`
- Create: `sntl-macros/src/relation/ir.rs`
- Modify: `sntl-macros/src/lib.rs`

**Step 1:** This is a proc-macro parsing step — test via trybuild in Task 6.

**Step 2: Implement relation IR parsing**

Create `sntl-macros/src/relation/ir.rs`:

```rust
use syn::{Ident, Type, ImplItem, ReturnType};
use quote::quote;

#[derive(Debug)]
pub struct RelationIR {
    pub model_name: Ident,
    pub relations: Vec<SingleRelationIR>,
}

#[derive(Debug)]
pub struct SingleRelationIR {
    pub fn_name: Ident,
    pub const_name: Ident,        // UPPERCASE version
    pub kind: RelationKindIR,
    pub target_type: Type,
    pub foreign_key: String,
}

#[derive(Debug, Clone, Copy)]
pub enum RelationKindIR {
    HasMany,
    HasOne,
    BelongsTo,
}

impl RelationIR {
    pub fn parse(item_impl: &syn::ItemImpl) -> syn::Result<Self> {
        let model_name = match &*item_impl.self_ty {
            Type::Path(tp) => tp.path.segments.last().unwrap().ident.clone(),
            _ => return Err(syn::Error::new_spanned(&item_impl.self_ty, "expected a type name")),
        };

        let mut relations = Vec::new();

        for item in &item_impl.items {
            if let ImplItem::Fn(method) = item {
                let fn_name = &method.sig.ident;

                // Parse return type: HasMany<T>, HasOne<T>, or BelongsTo<T>
                let (kind, target_type) = parse_return_type(&method.sig.output)?;

                // Parse body for foreign_key string literal
                let foreign_key = extract_foreign_key(&method.block)?;

                let const_name = Ident::new(
                    &fn_name.to_string().to_uppercase(),
                    fn_name.span(),
                );

                relations.push(SingleRelationIR {
                    fn_name: fn_name.clone(),
                    const_name,
                    kind,
                    target_type,
                    foreign_key,
                });
            }
        }

        Ok(RelationIR { model_name, relations })
    }
}

fn parse_return_type(ret: &ReturnType) -> syn::Result<(RelationKindIR, Type)> {
    // Extract from -> HasMany<Post>, -> HasOne<Profile>, -> BelongsTo<User>
    match ret {
        ReturnType::Type(_, ty) => {
            if let Type::Path(tp) = ty.as_ref() {
                if let Some(seg) = tp.path.segments.last() {
                    let kind = match seg.ident.to_string().as_str() {
                        "HasMany" => RelationKindIR::HasMany,
                        "HasOne" => RelationKindIR::HasOne,
                        "BelongsTo" => RelationKindIR::BelongsTo,
                        other => return Err(syn::Error::new_spanned(
                            &seg.ident,
                            format!("expected HasMany, HasOne, or BelongsTo, got `{other}`"),
                        )),
                    };
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(target)) = args.args.first() {
                            return Ok((kind, target.clone()));
                        }
                    }
                }
            }
            Err(syn::Error::new_spanned(ty, "expected HasMany<T>, HasOne<T>, or BelongsTo<T>"))
        }
        _ => Err(syn::Error::new_spanned(ret, "relation function must have a return type")),
    }
}

fn extract_foreign_key(block: &syn::Block) -> syn::Result<String> {
    // Look for string literal in ::new("foreign_key")
    for stmt in &block.stmts {
        let s = quote!(#stmt).to_string();
        if let Some(start) = s.find("\"") {
            if let Some(end) = s[start+1..].find("\"") {
                return Ok(s[start+1..start+1+end].to_string());
            }
        }
    }
    Err(syn::Error::new_spanned(block, "could not extract foreign key string from relation body"))
}
```

Create `sntl-macros/src/relation/mod.rs`:
```rust
pub mod ir;
pub mod codegen;
```

**Step 3: Implement `#[sentinel(relations)]` attribute macro**

Add to `sntl-macros/src/lib.rs`:

```rust
mod relation;

/// Declare relations on a model.
///
/// ```rust,ignore
/// #[sentinel(relations)]
/// impl User {
///     pub fn posts() -> HasMany<Post> { HasMany::new("user_id") }
/// }
/// ```
#[proc_macro_attribute]
pub fn sentinel(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_str = attr.to_string();
    if attr_str.trim() == "relations" {
        relation::expand_relations(item.into()).into()
    } else {
        // Future: other sentinel attributes
        item
    }
}
```

**Step 4:** Run `cargo check -p sntl-macros` → PASS (compiles)

**Step 5:** Commit `feat(macros): add relation IR parsing for #[sentinel(relations)]`

---

### Task 5: Relation Codegen — Type-State Struct + Accessors

**Files:**
- Create: `sntl-macros/src/relation/codegen.rs`
- Modify: `sntl-macros/src/relation/mod.rs`

**Step 1: Implement codegen**

`sntl-macros/src/relation/codegen.rs`:

```rust
use proc_macro2::TokenStream;
use quote::{quote, format_ident};
use super::ir::{RelationIR, SingleRelationIR, RelationKindIR};

pub fn generate_relations(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;
    let relation_consts = generate_relation_constants(ir);
    let pascal_methods = generate_pascal_find_methods(ir);
    // Type-state struct modification and accessors are generated
    // as a wrapper struct: Model_WithRelations<...>
    // For Phase 4 MVP: use the ModelQuery wrapper approach

    quote! {
        #relation_consts
        #pascal_methods
    }
}

fn generate_relation_constants(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;
    let consts: Vec<TokenStream> = ir.relations.iter().map(|rel| {
        let const_name = &rel.const_name;
        let fk = &rel.foreign_key;
        let target_table = infer_table_name(&rel.target_type);
        let kind_token = match rel.kind {
            RelationKindIR::HasMany => quote!(sntl::core::relation::RelationKind::HasMany),
            RelationKindIR::HasOne => quote!(sntl::core::relation::RelationKind::HasOne),
            RelationKindIR::BelongsTo => quote!(sntl::core::relation::RelationKind::BelongsTo),
        };
        quote! {
            pub const #const_name: sntl::core::relation::RelationSpec =
                sntl::core::relation::RelationSpec::new_const(
                    stringify!(#const_name),
                    #fk,
                    #target_table,
                    #kind_token,
                );
        }
    }).collect();

    quote! {
        #[automatically_derived]
        impl #model {
            #(#consts)*
        }
    }
}

fn generate_pascal_find_methods(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;
    let table = infer_table_name_from_ident(model);

    quote! {
        #[automatically_derived]
        impl #model {
            /// Start a SELECT query (PascalCase API).
            #[allow(non_snake_case)]
            pub fn Find() -> sntl::core::query::ModelQuery {
                sntl::core::query::ModelQuery::from_table(#table)
            }

            /// SELECT by primary key (PascalCase API).
            #[allow(non_snake_case)]
            pub fn FindId(id: impl Into<sntl::core::Value>) -> sntl::core::query::ModelQuery {
                let expr = sntl::core::Expr::Compare {
                    column: format!("\"{}\"", Self::PRIMARY_KEY),
                    op: "=",
                    value: id.into(),
                };
                sntl::core::query::ModelQuery::from_table(#table).Where(expr)
            }
        }
    }
}

fn infer_table_name(ty: &syn::Type) -> String {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let name = seg.ident.to_string();
            return format!("{}s", to_snake_case(&name));
        }
    }
    "unknown".to_string()
}

fn infer_table_name_from_ident(ident: &syn::Ident) -> String {
    format!("{}s", to_snake_case(&ident.to_string()))
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 { result.push('_'); }
        result.push(ch.to_ascii_lowercase());
    }
    result
}
```

Add to `sntl-macros/src/relation/mod.rs`:

```rust
pub mod ir;
pub mod codegen;

use proc_macro2::TokenStream;

pub fn expand_relations(input: TokenStream) -> TokenStream {
    let item_impl = match syn::parse2::<syn::ItemImpl>(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    let ir = match ir::RelationIR::parse(&item_impl) {
        Ok(ir) => ir,
        Err(e) => return e.to_compile_error(),
    };

    codegen::generate_relations(&ir)
}
```

**Step 2:** Run `cargo check --workspace` → PASS

**Step 3:** Commit `feat(macros): generate relation constants and PascalCase Find/FindId methods`

---

### Task 6: End-to-End Test — Relations Compile + Run

**Files:**
- Create: `sntl/tests/relation_e2e_test.rs`

**Step 1: Write test**

```rust
use sntl::prelude::*;
use sntl::core::relation::*;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Model)]
#[sentinel(table = "posts")]
pub struct Post {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub title: String,
}

#[sentinel(relations)]
impl User {
    pub fn posts() -> HasMany<Post> { HasMany::new("user_id") }
}

#[test]
fn relation_constant_exists() {
    let spec = User::POSTS;
    assert_eq!(spec.foreign_key(), "user_id");
    assert_eq!(spec.target_table(), "posts");
}

#[test]
fn find_builds_select() {
    let (sql, _) = User::Find().Build();
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("users"));
}

#[test]
fn find_id_builds_where() {
    let (sql, binds) = User::FindId(42i32).Build();
    assert!(sql.contains("WHERE"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn relation_spec_filter_order_limit() {
    let spec = User::POSTS
        .Filter(Post::PUBLISHED.eq(true))
        .Limit(5);
    assert!(spec.has_filters());
    assert_eq!(spec.limit(), Some(5));
}

#[test]
fn batch_sql_generation() {
    let spec = User::POSTS;
    let (sql, binds) = spec.build_batch_sql(&[1i32.into(), 2i32.into()]);
    assert!(sql.contains("WHERE \"user_id\" IN ($1, $2)"));
    assert_eq!(binds.len(), 2);
}
```

**Step 2:** Run `cargo test -p sntl --test relation_e2e_test` → should PASS if Tasks 1-5 done correctly

**Step 3:** Fix any issues

**Step 4:** Commit `test: add end-to-end relation declaration and query tests`

---

### Task 7: FetchAll/FetchOne on ModelQuery (Execution)

**Files:**
- Modify: `sntl/src/core/query/pascal.rs`

**Step 1: Add execution methods to ModelQuery**

```rust
impl ModelQuery {
    #[allow(non_snake_case)]
    pub async fn FetchAll(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<Vec<driver::Row>> {
        self.inner.fetch_all(conn).await
    }

    #[allow(non_snake_case)]
    pub async fn FetchOne(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<driver::Row> {
        self.inner.fetch_one(conn).await
    }

    #[allow(non_snake_case)]
    pub async fn FetchOptional(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<Option<driver::Row>> {
        self.inner.fetch_optional(conn).await
    }

    #[allow(non_snake_case)]
    pub async fn FetchStream(
        self,
        conn: &mut driver::Connection,
    ) -> crate::core::error::Result<driver::RowStream<'_>> {
        self.inner.fetch_stream(conn).await
    }
}
```

**Step 2:** Run `cargo check --workspace` → PASS

**Step 3:** Commit `feat(core): add FetchAll/FetchOne/FetchOptional/FetchStream to ModelQuery`

---

### Task 8: Integration Test — Relations with Live PG

**Files:**
- Create: `sntl/tests/pg_relation_test.rs`

**Step 1: Write integration test**

```rust
#[macro_use]
mod pg_helpers;

use sntl::prelude::*;

// (reuse User/Post models from Task 6, or define inline)

#[tokio::test]
async fn pascal_find_fetch_all() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert test data
    InsertQuery::new("users")
        .column("name", "Alice")
        .column("email", "alice@test.com")
        .no_returning()
        .execute(&mut conn).await.unwrap();

    let rows = User::Find().FetchAll(&mut conn).await.unwrap();
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn batch_load_generates_where_in() {
    let url = require_pg!();
    let config = sntl::driver::Config::parse(&url).unwrap();
    let mut conn = sntl::driver::Connection::connect(config).await.unwrap();
    pg_helpers::clean_tables(&mut conn).await;

    // Insert user + posts
    let user_rows = InsertQuery::new("users")
        .column("name", "Alice")
        .column("email", "alice@test.com")
        .fetch_returning(&mut conn).await.unwrap();
    let user_id: i32 = user_rows[0].get(0);

    InsertQuery::new("posts")
        .column("user_id", user_id)
        .column("title", "Post 1")
        .column("body", "")
        .no_returning()
        .execute(&mut conn).await.unwrap();

    InsertQuery::new("posts")
        .column("user_id", user_id)
        .column("title", "Post 2")
        .column("body", "")
        .no_returning()
        .execute(&mut conn).await.unwrap();

    // Batch load posts for user
    let spec = User::POSTS;
    let (sql, binds) = spec.build_batch_sql(&[user_id.into()]);
    let post_rows = conn.query(&sql, &binds.iter().map(|v| v as &(dyn sntl::driver::ToSql + Sync)).collect::<Vec<_>>()).await.unwrap();
    assert_eq!(post_rows.len(), 2);
}
```

**Step 2:** Run `cargo test -p sntl --test pg_relation_test` → PASS (skips without DATABASE_URL)

**Step 3:** Commit `test: add integration tests for PascalCase queries and batch loading`

---

### Task 9: Update Codecov Ignore + Final Verification

**Files:**
- Modify: `.github/workflows/codecov.yml` (add `relation\.rs|pascal\.rs` if they contain async-only methods)
- Run full CI locally

**Step 1:** Run full check

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**Step 2:** Fix any issues

**Step 3:** Commit `chore: Phase 4 cleanup and CI fixes`

**Step 4:** Push branch, create PR

---

## Implementation Order Summary

| Task | What | Dependencies |
|------|------|-------------|
| 1 | Core relation types (HasMany, HasOne, BelongsTo) | None |
| 2 | RelationSpec (Filter, OrderBy, Limit, batch SQL) | Task 1 |
| 3 | PascalCase ModelQuery wrapper | None |
| 4 | Relation IR parsing in proc macros | None |
| 5 | Relation codegen (constants, Find/FindId) | Task 4 |
| 6 | End-to-end compile test | Tasks 1-5 |
| 7 | FetchAll/FetchOne execution on ModelQuery | Task 3 |
| 8 | Integration tests with live PG | Tasks 6-7 |
| 9 | CI cleanup + PR | Task 8 |

**Parallel tracks:** Tasks 1-2 (core types) and Tasks 3-4 (query wrapper + macro IR) can run in parallel.

**NOTE:** This is Phase 4 MVP — type-state struct generation (the `User<Posts, Profile>` generics) is designed but not implemented in this plan. The MVP delivers: relation constants, PascalCase API, batch SQL generation, and filtered includes. The type-state generic struct generation will be Phase 4.1 once the foundation is solid.
