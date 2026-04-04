# Sentinel ORM — Phase 2: Derive Macros Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `derive(Model)` and `derive(Partial)` proc macros that generate type-safe, zero-cost code from annotated Rust structs.

**Architecture:** Phase 2 builds `sentinel-macros` as a proc-macro crate that parses `#[sentinel(...)]` attributes using darling, constructs an intermediate representation (IR), then generates code via quote. The macro depends on types from `sentinel-core` (Model trait, Column, InsertQuery, etc.) which were built in Phase 1. Tests use a separate integration test crate pattern since proc-macro crates cannot have integration tests directly.

**Tech Stack:** Rust (stable), syn 2, quote, darling, proc-macro-error2

**Design Doc:** `docs/plans/2026-04-04-sentinel-phase2-design.md`

---

## Task 1: Add Macro Dependencies

**Files:**
- Modify: `sentinel-macros/Cargo.toml`
- Modify: `Cargo.toml` (workspace dependencies)
- Modify: `sentinel-core/Cargo.toml` (add sentinel-macros dependency for re-export)
- Modify: `sentinel-core/src/lib.rs` (re-export derive macros)

**Step 1: Update workspace Cargo.toml with new dependencies**

Add to `[workspace.dependencies]` section in root `Cargo.toml`:
```toml
syn = { version = "2", features = ["full", "extra-traits"] }
quote = "1"
darling = "0.20"
proc-macro2 = "1"
proc-macro-error2 = "2"
```

**Step 2: Update sentinel-macros/Cargo.toml**

```toml
[package]
name = "sentinel-macros"
version.workspace = true
edition.workspace = true

[lib]
proc-macro = true

[dependencies]
syn.workspace = true
quote.workspace = true
darling.workspace = true
proc-macro2.workspace = true
proc-macro-error2.workspace = true
```

**Step 3: Add sentinel-macros as dependency of sentinel-core for re-export**

In `sentinel-core/Cargo.toml`, add to `[dependencies]`:
```toml
sentinel-macros.workspace = true
```

**Step 4: Re-export derive macros from sentinel-core**

Update `sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod model;
pub mod prelude;
pub mod query;
pub mod types;

pub use error::{Error, Result};

// Re-export derive macros so users write `use sentinel_core::Model;`
pub use sentinel_macros::Model;
```

Note: `Partial` re-export will be added in Task 7 when derive(Partial) is implemented.

**Step 5: Verify workspace compiles**

```bash
cargo check --workspace
```

Expected: All crates compile with zero errors.

**Step 6: Commit**

```bash
git add Cargo.toml sentinel-macros/Cargo.toml sentinel-core/Cargo.toml sentinel-core/src/lib.rs
git commit -m "chore: add proc-macro dependencies (darling, syn, quote)"
```

---

## Task 2: ModelIR — Parse #[derive(Model)] Attributes

**Files:**
- Create: `sentinel-macros/src/model/mod.rs`
- Create: `sentinel-macros/src/model/ir.rs`
- Modify: `sentinel-macros/src/lib.rs`

**Step 1: Write the stub lib.rs with derive entry point**

`sentinel-macros/src/lib.rs`:
```rust
//! Sentinel Macros — derive(Model), derive(Partial), #[reducer].

mod model;

use proc_macro::TokenStream;

/// Derive the `Model` trait for a struct.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Model)]
/// #[sentinel(table = "users")]
/// pub struct User {
///     #[sentinel(primary_key, default = "gen_random_uuid()")]
///     pub id: Uuid,
///     pub email: String,
/// }
/// ```
#[proc_macro_derive(Model, attributes(sentinel))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    model::derive_model_impl(input.into()).into()
}
```

**Step 2: Create the IR module with darling parsing**

`sentinel-macros/src/model/ir.rs`:
```rust
use darling::{FromDeriveInput, FromField};
use syn::{Ident, Type};

/// Parsed struct-level attributes from `#[sentinel(...)]`.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(sentinel), supports(struct_named))]
pub struct ModelOpts {
    pub ident: Ident,
    pub data: darling::ast::Data<(), FieldOpts>,

    /// Table name override. If None, inferred from struct name.
    #[darling(default)]
    pub table: Option<String>,
}

/// Parsed field-level attributes from `#[sentinel(...)]`.
#[derive(Debug, FromField)]
#[darling(attributes(sentinel))]
pub struct FieldOpts {
    pub ident: Option<Ident>,
    pub ty: Type,

    /// Marks this field as the primary key.
    #[darling(default)]
    pub primary_key: bool,

    /// SQL default expression (e.g., "now()"). Field will be skipped in NewModel.
    #[darling(default)]
    pub default: Option<String>,

    /// Column name override if different from field name.
    #[darling(default)]
    pub column: Option<String>,

    /// Marks column as unique (metadata for migrations).
    #[darling(default)]
    pub unique: bool,

    /// Skip this field entirely (not a DB column).
    #[darling(default)]
    pub skip: bool,
}

/// Processed intermediate representation for code generation.
#[derive(Debug)]
pub struct ModelIR {
    pub struct_name: Ident,
    pub table_name: String,
    pub fields: Vec<FieldIR>,
    pub primary_key_index: usize,
}

#[derive(Debug)]
pub struct FieldIR {
    pub field_name: Ident,
    pub column_name: String,
    pub rust_type: Type,
    pub column_type: &'static str,
    pub nullable: bool,
    pub has_default: bool,
    pub is_primary_key: bool,
    pub skip: bool,
}

impl ModelOpts {
    /// Convert parsed darling opts into the codegen IR.
    pub fn into_ir(self) -> Result<ModelIR, darling::Error> {
        let struct_name = self.ident;

        // Infer table name: snake_case + pluralize (simple "s" suffix)
        let table_name = self.table.unwrap_or_else(|| {
            let snake = to_snake_case(&struct_name.to_string());
            format!("{snake}s")
        });

        let fields_data = self
            .data
            .take_struct()
            .expect("darling supports(struct_named) ensures this");

        let mut fields = Vec::new();
        let mut pk_index = None;

        for (i, f) in fields_data.fields.into_iter().enumerate() {
            if f.skip {
                fields.push(FieldIR {
                    field_name: f.ident.clone().unwrap(),
                    column_name: String::new(),
                    rust_type: f.ty.clone(),
                    column_type: "",
                    nullable: false,
                    has_default: false,
                    is_primary_key: false,
                    skip: true,
                });
                continue;
            }

            let field_name = f.ident.clone().unwrap();
            let column_name = f.column.unwrap_or_else(|| field_name.to_string());
            let nullable = is_option_type(&f.ty);
            let column_type = rust_type_to_column_type(&f.ty);
            let has_default = f.default.is_some();

            if f.primary_key {
                if pk_index.is_some() {
                    return Err(darling::Error::custom("only one field can be marked #[sentinel(primary_key)]")
                        .with_span(&field_name));
                }
                pk_index = Some(i);
            }

            fields.push(FieldIR {
                field_name,
                column_name,
                rust_type: f.ty,
                column_type,
                nullable,
                has_default,
                is_primary_key: f.primary_key,
                skip: false,
            });
        }

        let primary_key_index = pk_index.ok_or_else(|| {
            darling::Error::custom("no field marked with #[sentinel(primary_key)] — add it to exactly one field")
                .with_span(&struct_name)
        })?;

        Ok(ModelIR {
            struct_name,
            table_name,
            fields,
            primary_key_index,
        })
    }
}

/// Convert `CamelCase` to `snake_case`.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Check if a type is `Option<T>`.
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

/// Map Rust types to PostgreSQL column type strings.
fn rust_type_to_column_type(ty: &Type) -> &'static str {
    let type_str = extract_type_name(ty);
    match type_str.as_str() {
        "String" => "text",
        "i32" => "int4",
        "i64" => "int8",
        "f64" => "float8",
        "bool" => "bool",
        "Uuid" => "uuid",
        "DateTime" => "timestamptz",
        "Vec" => "bytea", // Vec<u8>
        _ => "text", // fallback
    }
}

/// Extract the outermost type name, unwrapping Option<T> if present.
fn extract_type_name(ty: &Type) -> String {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                // Unwrap Option<T> and get T's name
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return extract_type_name(inner);
                    }
                }
            }
            return seg.ident.to_string();
        }
    }
    "unknown".to_string()
}
```

**Step 3: Create the model module with stub codegen**

`sentinel-macros/src/model/mod.rs`:
```rust
pub mod ir;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;

use ir::ModelOpts;

pub fn derive_model_impl(input: TokenStream) -> TokenStream {
    let derive_input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(di) => di,
        Err(e) => return e.to_compile_error(),
    };

    let opts = match ModelOpts::from_derive_input(&derive_input) {
        Ok(o) => o,
        Err(e) => return e.write_errors(),
    };

    let ir = match opts.into_ir() {
        Ok(ir) => ir,
        Err(e) => return e.write_errors(),
    };

    // Stub: just generate an empty impl to verify parsing works
    let name = &ir.struct_name;
    let table = &ir.table_name;

    quote! {
        impl #name {
            /// Table name (temporary stub — full codegen in next task).
            pub const __TABLE: &'static str = #table;
        }
    }
}
```

**Step 4: Verify workspace compiles**

```bash
cargo check --workspace
```

Expected: Compiles with zero errors.

**Step 5: Commit**

```bash
git add sentinel-macros/src/
git commit -m "feat(macros): add ModelIR with darling attribute parsing"
```

---

## Task 3: Codegen — Model Trait Implementation

**Files:**
- Create: `sentinel-macros/src/model/codegen.rs`
- Modify: `sentinel-macros/src/model/mod.rs`
- Create: `sentinel-core/tests/derive_model_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/derive_model_test.rs`:
```rust
use sentinel_core::expr::Column;
use sentinel_core::model::{Model, ModelColumn};
use sentinel_core::types::Value;
use sentinel_core::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,

    #[sentinel(unique)]
    pub email: String,

    pub name: Option<String>,

    #[sentinel(default = "now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[test]
fn model_trait_table_name() {
    assert_eq!(User::TABLE, "users");
}

#[test]
fn model_trait_primary_key() {
    assert_eq!(User::PRIMARY_KEY, "id");
}

#[test]
fn model_trait_columns() {
    let cols = User::columns();
    assert_eq!(cols.len(), 4);
    assert_eq!(cols[0].name, "id");
    assert_eq!(cols[0].column_type, "uuid");
    assert!(!cols[0].nullable);
    assert!(cols[0].has_default);

    assert_eq!(cols[1].name, "email");
    assert!(!cols[1].nullable);
    assert!(!cols[1].has_default);

    assert_eq!(cols[2].name, "name");
    assert!(cols[2].nullable);

    assert_eq!(cols[3].name, "created_at");
    assert!(cols[3].has_default);
}

#[test]
fn model_find() {
    let q = User::find();
    let (sql, _) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
}

#[test]
fn model_find_by_id() {
    let q = User::find_by_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\" WHERE \"id\" = $1");
    assert_eq!(binds.len(), 1);
}

#[test]
fn model_delete() {
    let q = User::delete(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1");
    assert_eq!(binds.len(), 1);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test derive_model_test
```

Expected: FAIL — stub codegen doesn't implement Model trait yet.

**Step 3: Write the codegen module**

`sentinel-macros/src/model/codegen.rs`:
```rust
use proc_macro2::TokenStream;
use quote::quote;

use super::ir::ModelIR;

/// Generate the `Model` trait implementation.
pub fn generate_model_impl(ir: &ModelIR) -> TokenStream {
    let name = &ir.struct_name;
    let table = &ir.table_name;
    let pk_field = &ir.fields[ir.primary_key_index];
    let pk_name = &pk_field.column_name;

    let column_entries: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let col_name = &f.column_name;
            let col_type = f.column_type;
            let nullable = f.nullable;
            let has_default = f.has_default;
            quote! {
                sentinel_core::model::ModelColumn {
                    name: #col_name,
                    column_type: #col_type,
                    nullable: #nullable,
                    has_default: #has_default,
                }
            }
        })
        .collect();

    let num_columns = column_entries.len();

    quote! {
        #[automatically_derived]
        impl sentinel_core::model::Model for #name {
            const TABLE: &'static str = #table;
            const PRIMARY_KEY: &'static str = #pk_name;

            fn columns() -> &'static [sentinel_core::model::ModelColumn] {
                static COLUMNS: [sentinel_core::model::ModelColumn; #num_columns] = [
                    #(#column_entries),*
                ];
                &COLUMNS
            }
        }
    }
}
```

**Step 4: Update model/mod.rs to use codegen**

Replace `sentinel-macros/src/model/mod.rs`:
```rust
pub mod codegen;
pub mod ir;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;

use ir::ModelOpts;

pub fn derive_model_impl(input: TokenStream) -> TokenStream {
    let derive_input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(di) => di,
        Err(e) => return e.to_compile_error(),
    };

    let opts = match ModelOpts::from_derive_input(&derive_input) {
        Ok(o) => o,
        Err(e) => return e.write_errors(),
    };

    let ir = match opts.into_ir() {
        Ok(ir) => ir,
        Err(e) => return e.write_errors(),
    };

    let model_impl = codegen::generate_model_impl(&ir);
    let column_consts = codegen::generate_column_consts(&ir);

    quote::quote! {
        #model_impl
        #column_consts
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test derive_model_test
```

Expected: 6 tests PASS.

**Step 6: Commit**

```bash
git add sentinel-macros/src/ sentinel-core/tests/derive_model_test.rs
git commit -m "feat(macros): generate Model trait impl from derive(Model)"
```

---

## Task 4: Codegen — Column Constants

**Files:**
- Modify: `sentinel-macros/src/model/codegen.rs`
- Create: `sentinel-core/tests/derive_columns_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/derive_columns_test.rs`:
```rust
use sentinel_core::Model;

#[derive(Model)]
#[sentinel(table = "posts")]
pub struct Post {
    #[sentinel(primary_key)]
    pub id: i64,

    pub title: String,

    pub body: String,

    #[sentinel(default = "false")]
    pub published: bool,
}

#[test]
fn column_constant_id() {
    let expr = Post::ID.eq(42i64);
    assert_eq!(expr.to_sql(1), "\"posts\".\"id\" = $1");
}

#[test]
fn column_constant_title() {
    let expr = Post::TITLE.eq("Hello");
    assert_eq!(expr.to_sql(1), "\"posts\".\"title\" = $1");
}

#[test]
fn column_constant_published() {
    let expr = Post::PUBLISHED.eq(true);
    assert_eq!(expr.to_sql(1), "\"posts\".\"published\" = $1");
}

#[test]
fn column_constants_compose() {
    let expr = Post::TITLE.like("%rust%").and(Post::PUBLISHED.eq(true));
    let sql = expr.to_sql(1);
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));
}

#[test]
fn column_constants_in_select() {
    let q = Post::find().where_(Post::PUBLISHED.eq(true));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"posts\".* FROM \"posts\" WHERE \"posts\".\"published\" = $1"
    );
    assert_eq!(binds.len(), 1);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test derive_columns_test
```

Expected: FAIL — `Post::ID`, `Post::TITLE` etc. don't exist yet.

**Step 3: Add generate_column_consts to codegen.rs**

Add to `sentinel-macros/src/model/codegen.rs`:
```rust
/// Generate column constants as inherent `impl` methods.
pub fn generate_column_consts(ir: &ModelIR) -> TokenStream {
    let name = &ir.struct_name;
    let table = &ir.table_name;

    let consts: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let const_name = syn::Ident::new(
                &f.field_name.to_string().to_uppercase(),
                f.field_name.span(),
            );
            let col_name = &f.column_name;
            quote! {
                pub const #const_name: sentinel_core::expr::Column = sentinel_core::expr::Column {
                    table: std::borrow::Cow::Borrowed(#table),
                    name: std::borrow::Cow::Borrowed(#col_name),
                };
            }
        })
        .collect();

    quote! {
        #[automatically_derived]
        impl #name {
            #(#consts)*
        }
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test derive_columns_test
```

Expected: 5 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-macros/src/model/codegen.rs sentinel-core/tests/derive_columns_test.rs
git commit -m "feat(macros): generate Column constants from derive(Model)"
```

---

## Task 5: Codegen — NewModel Struct + create()

**Files:**
- Modify: `sentinel-macros/src/model/codegen.rs`
- Modify: `sentinel-macros/src/model/mod.rs`
- Create: `sentinel-core/tests/derive_create_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/derive_create_test.rs`:
```rust
use sentinel_core::types::Value;
use sentinel_core::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,

    #[sentinel(unique)]
    pub email: String,

    pub name: Option<String>,

    #[sentinel(default = "now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[test]
fn new_user_has_correct_fields() {
    // NewUser should only have email and name (id and created_at have defaults)
    let new = NewUser {
        email: "alice@example.com".to_string(),
        name: Some("Alice".to_string()),
    };
    assert_eq!(new.email, "alice@example.com");
    assert_eq!(new.name, Some("Alice".to_string()));
}

#[test]
fn create_builds_insert_query() {
    let new = NewUser {
        email: "alice@example.com".to_string(),
        name: Some("Alice".to_string()),
    };
    let q = User::create(new);
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\", \"name\") VALUES ($1, $2) RETURNING *"
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0], Value::Text("alice@example.com".into()));
}

#[test]
fn create_with_none_optional() {
    let new = NewUser {
        email: "bob@example.com".to_string(),
        name: None,
    };
    let q = User::create(new);
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\", \"name\") VALUES ($1, $2) RETURNING *"
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[1], Value::Null);
}

#[test]
fn all_default_fields_model() {
    // A model where only the PK has a default
    #[derive(Model)]
    #[sentinel(table = "tags")]
    pub struct Tag {
        #[sentinel(primary_key, default = "gen_random_uuid()")]
        pub id: uuid::Uuid,
        pub label: String,
    }

    let new = NewTag {
        label: "rust".to_string(),
    };
    let q = Tag::create(new);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"tags\" (\"label\") VALUES ($1) RETURNING *"
    );
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test derive_create_test
```

Expected: FAIL — `NewUser` struct and `User::create()` don't exist yet.

**Step 3: Add generate_new_struct and generate_create_method to codegen.rs**

Add to `sentinel-macros/src/model/codegen.rs`:
```rust
/// Generate the `New<Model>` struct for INSERT (skips fields with `default`).
pub fn generate_new_struct(ir: &ModelIR) -> TokenStream {
    let new_name = syn::Ident::new(
        &format!("New{}", ir.struct_name),
        ir.struct_name.span(),
    );

    let fields: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.skip && !f.has_default)
        .map(|f| {
            let name = &f.field_name;
            let ty = &f.rust_type;
            quote! { pub #name: #ty }
        })
        .collect();

    quote! {
        #[automatically_derived]
        pub struct #new_name {
            #(#fields),*
        }
    }
}

/// Generate the `create(new) -> InsertQuery` method.
pub fn generate_create_method(ir: &ModelIR) -> TokenStream {
    let struct_name = &ir.struct_name;
    let new_name = syn::Ident::new(
        &format!("New{}", ir.struct_name),
        ir.struct_name.span(),
    );
    let table = &ir.table_name;

    let column_calls: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.skip && !f.has_default)
        .map(|f| {
            let col_name = &f.column_name;
            let field_name = &f.field_name;
            quote! { .column(#col_name, new.#field_name) }
        })
        .collect();

    quote! {
        #[automatically_derived]
        impl #struct_name {
            pub fn create(new: #new_name) -> sentinel_core::query::InsertQuery {
                sentinel_core::query::InsertQuery::new(#table)
                    #(#column_calls)*
            }
        }
    }
}
```

**Step 4: Update model/mod.rs to call all generators**

Replace `sentinel-macros/src/model/mod.rs`:
```rust
pub mod codegen;
pub mod ir;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;

use ir::ModelOpts;

pub fn derive_model_impl(input: TokenStream) -> TokenStream {
    let derive_input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(di) => di,
        Err(e) => return e.to_compile_error(),
    };

    let opts = match ModelOpts::from_derive_input(&derive_input) {
        Ok(o) => o,
        Err(e) => return e.write_errors(),
    };

    let ir = match opts.into_ir() {
        Ok(ir) => ir,
        Err(e) => return e.write_errors(),
    };

    let model_impl = codegen::generate_model_impl(&ir);
    let column_consts = codegen::generate_column_consts(&ir);
    let new_struct = codegen::generate_new_struct(&ir);
    let create_method = codegen::generate_create_method(&ir);

    quote::quote! {
        #model_impl
        #column_consts
        #new_struct
        #create_method
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test derive_create_test
```

Expected: 4 tests PASS.

**Step 6: Commit**

```bash
git add sentinel-macros/src/ sentinel-core/tests/derive_create_test.rs
git commit -m "feat(macros): generate NewModel struct and create() from derive(Model)"
```

---

## Task 6: Table Name Inference + Column Rename + Skip

**Files:**
- Create: `sentinel-core/tests/derive_attrs_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/derive_attrs_test.rs`:
```rust
use sentinel_core::model::Model;
use sentinel_core::Model as DeriveModel;

// Test: table name inferred from struct name (UserProfile → "user_profiles")
#[derive(DeriveModel)]
pub struct UserProfile {
    #[sentinel(primary_key)]
    pub id: i64,
    pub display_name: String,
}

#[test]
fn inferred_table_name() {
    assert_eq!(UserProfile::TABLE, "user_profiles");
}

// Test: column rename
#[derive(DeriveModel)]
#[sentinel(table = "items")]
pub struct Item {
    #[sentinel(primary_key)]
    pub id: i64,

    #[sentinel(column = "item_name")]
    pub name: String,
}

#[test]
fn column_rename() {
    let expr = Item::NAME.eq("Widget");
    assert_eq!(expr.to_sql(1), "\"items\".\"item_name\" = $1");
}

#[test]
fn column_rename_in_metadata() {
    let cols = Item::columns();
    assert_eq!(cols[1].name, "item_name");
}

// Test: skip field
#[derive(DeriveModel)]
#[sentinel(table = "products")]
pub struct Product {
    #[sentinel(primary_key)]
    pub id: i64,

    pub sku: String,

    #[sentinel(skip)]
    pub computed_label: String,
}

#[test]
fn skip_field_not_in_columns() {
    let cols = Product::columns();
    assert_eq!(cols.len(), 2); // id + sku, not computed_label
}

#[test]
fn skip_field_no_column_constant() {
    // Product should have ID and SKU constants but NOT COMPUTED_LABEL
    let _ = Product::ID;
    let _ = Product::SKU;
    // Product::COMPUTED_LABEL should not exist — compile error if uncommented
}

#[test]
fn skip_field_not_in_new_struct() {
    // NewProduct should only have sku (id has no default but is PK... wait, id has no default here)
    // Actually: id has no default, so NewProduct has id + sku (not computed_label)
    let new = NewProduct {
        id: 1,
        sku: "ABC-123".to_string(),
    };
    assert_eq!(new.sku, "ABC-123");
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test derive_attrs_test
```

Expected: FAIL — table name inference doesn't pluralize correctly for multi-word names.

**Step 3: Fix to_snake_case pluralization for multi-word names**

Update `to_snake_case` in `sentinel-macros/src/model/ir.rs` — the current implementation already handles this. The inferred table for `UserProfile` would be `user_profiles` (snake_case + "s"). Verify by checking:
- `UserProfile` → `to_snake_case("UserProfile")` → `"user_profile"` → `"user_profiles"`

If the test passes without changes, great. If not, debug the snake_case logic.

**Step 4: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test derive_attrs_test
```

Expected: 6 tests PASS.

**Step 5: Commit**

```bash
git add sentinel-core/tests/derive_attrs_test.rs
git commit -m "test(macros): verify table inference, column rename, skip attribute"
```

---

## Task 7: derive(Partial) — Parse and Validate

**Files:**
- Create: `sentinel-macros/src/partial/mod.rs`
- Create: `sentinel-macros/src/partial/ir.rs`
- Create: `sentinel-macros/src/partial/codegen.rs`
- Modify: `sentinel-macros/src/lib.rs`
- Modify: `sentinel-core/src/lib.rs` (re-export Partial derive)
- Modify: `sentinel-core/src/prelude.rs` (add Partial)
- Create: `sentinel-core/tests/derive_partial_test.rs`

**Step 1: Write the failing test**

`sentinel-core/tests/derive_partial_test.rs`:
```rust
use sentinel_core::types::Value;
use sentinel_core::{Model, Partial};

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,

    #[sentinel(unique)]
    pub email: String,

    pub name: Option<String>,

    #[sentinel(default = "now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Partial)]
#[sentinel(model = "User")]
pub struct UserSummary {
    pub id: uuid::Uuid,
    pub name: Option<String>,
}

#[test]
fn partial_select_query() {
    let q = UserSummary::select_query();
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"name\" FROM \"users\""
    );
    assert!(binds.is_empty());
}

#[test]
fn partial_select_with_where() {
    let q = UserSummary::select_query().where_(User::NAME.is_not_null());
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"name\" FROM \"users\" WHERE \"users\".\"name\" IS NOT NULL"
    );
}

#[test]
fn partial_select_with_limit() {
    let q = UserSummary::select_query().limit(10);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"name\" FROM \"users\" LIMIT 10"
    );
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sentinel-core --test derive_partial_test
```

Expected: FAIL — `Partial` derive doesn't exist yet.

**Step 3: Implement derive(Partial)**

`sentinel-macros/src/partial/ir.rs`:
```rust
use darling::{FromDeriveInput, FromField};
use syn::{Ident, Type};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(sentinel), supports(struct_named))]
pub struct PartialOpts {
    pub ident: Ident,
    pub data: darling::ast::Data<(), PartialFieldOpts>,

    /// The model this partial type selects from (e.g., "User").
    pub model: String,
}

#[derive(Debug, FromField)]
#[darling(attributes(sentinel))]
pub struct PartialFieldOpts {
    pub ident: Option<Ident>,
    pub ty: Type,
}

#[derive(Debug)]
pub struct PartialIR {
    pub struct_name: Ident,
    pub model_name: String,
    pub fields: Vec<PartialFieldIR>,
}

#[derive(Debug)]
pub struct PartialFieldIR {
    pub field_name: Ident,
    pub column_name: String,
}

impl PartialOpts {
    pub fn into_ir(self) -> Result<PartialIR, darling::Error> {
        let fields_data = self
            .data
            .take_struct()
            .expect("darling supports(struct_named) ensures this");

        let fields = fields_data
            .fields
            .into_iter()
            .map(|f| {
                let field_name = f.ident.clone().unwrap();
                let column_name = field_name.to_string();
                PartialFieldIR {
                    field_name,
                    column_name,
                }
            })
            .collect();

        Ok(PartialIR {
            struct_name: self.ident,
            model_name: self.model,
            fields,
        })
    }
}
```

`sentinel-macros/src/partial/codegen.rs`:
```rust
use proc_macro2::TokenStream;
use quote::quote;

use super::ir::PartialIR;

pub fn generate_partial_impl(ir: &PartialIR) -> TokenStream {
    let struct_name = &ir.struct_name;
    let model_ident = syn::Ident::new(&ir.model_name, struct_name.span());

    let column_names: Vec<&str> = ir.fields.iter().map(|f| f.column_name.as_str()).collect();
    let column_strs: Vec<&str> = column_names.iter().copied().collect();

    quote! {
        #[automatically_derived]
        impl #struct_name {
            /// Build a SELECT query that fetches only this partial type's columns.
            pub fn select_query() -> sentinel_core::query::SelectQuery {
                sentinel_core::query::SelectQuery::new(
                    <#model_ident as sentinel_core::model::Model>::TABLE
                )
                .columns(vec![#(#column_strs),*])
            }
        }
    }
}
```

`sentinel-macros/src/partial/mod.rs`:
```rust
pub mod codegen;
pub mod ir;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;

use ir::PartialOpts;

pub fn derive_partial_impl(input: TokenStream) -> TokenStream {
    let derive_input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(di) => di,
        Err(e) => return e.to_compile_error(),
    };

    let opts = match PartialOpts::from_derive_input(&derive_input) {
        Ok(o) => o,
        Err(e) => return e.write_errors(),
    };

    let ir = match opts.into_ir() {
        Ok(ir) => ir,
        Err(e) => return e.write_errors(),
    };

    codegen::generate_partial_impl(&ir)
}
```

**Step 4: Update sentinel-macros/src/lib.rs**

```rust
//! Sentinel Macros — derive(Model), derive(Partial), #[reducer].

mod model;
mod partial;

use proc_macro::TokenStream;

/// Derive the `Model` trait for a struct.
#[proc_macro_derive(Model, attributes(sentinel))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    model::derive_model_impl(input.into()).into()
}

/// Derive a partial select type.
#[proc_macro_derive(Partial, attributes(sentinel))]
pub fn derive_partial(input: TokenStream) -> TokenStream {
    partial::derive_partial_impl(input.into()).into()
}
```

**Step 5: Re-export Partial from sentinel-core**

Update `sentinel-core/src/lib.rs`:
```rust
//! Sentinel Core — Model trait, QueryBuilder, types, and connection abstraction.

pub mod error;
pub mod expr;
pub mod model;
pub mod prelude;
pub mod query;
pub mod types;

pub use error::{Error, Result};

// Re-export derive macros
pub use sentinel_macros::Model;
pub use sentinel_macros::Partial;
```

Update `sentinel-core/src/prelude.rs`:
```rust
//! Common imports for Sentinel users.
//!
//! ```rust
//! use sentinel_core::prelude::*;
//! ```

pub use crate::error::{Error, Result};
pub use crate::expr::{Column, Expr, OrderExpr};
pub use crate::model::{Model, ModelColumn};
pub use crate::query::{DeleteQuery, InsertQuery, QueryBuilder, SelectQuery, UpdateQuery};
pub use crate::types::Value;

// Re-export derive macros
pub use sentinel_macros::Model as DeriveModel;
pub use sentinel_macros::Partial as DerivePartial;
```

**Step 6: Run test to verify it passes**

```bash
cargo test -p sentinel-core --test derive_partial_test
```

Expected: 3 tests PASS.

**Step 7: Commit**

```bash
git add sentinel-macros/src/ sentinel-core/src/lib.rs sentinel-core/src/prelude.rs sentinel-core/tests/derive_partial_test.rs
git commit -m "feat(macros): add derive(Partial) for narrow select types"
```

---

## Task 8: Error Message Quality

**Files:**
- Create: `sentinel-macros/tests/compile_fail/` (trybuild tests)
- Create: `sentinel-macros/tests/error_test.rs`

This task tests that bad input produces helpful error messages, not cryptic trait bounds.

**Step 1: Add trybuild dependency**

Add to `[workspace.dependencies]` in root `Cargo.toml`:
```toml
trybuild = "1"
```

Add to `sentinel-macros/Cargo.toml`:
```toml
[dev-dependencies]
trybuild.workspace = true
sentinel-core.workspace = true
uuid = { workspace = true }
chrono = { workspace = true }
```

**Step 2: Create compile-fail test cases**

`sentinel-macros/tests/compile_fail/no_primary_key.rs`:
```rust
use sentinel_core::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    pub id: i64,
    pub email: String,
}

fn main() {}
```

`sentinel-macros/tests/compile_fail/no_primary_key.stderr`:
```
error: no field marked with #[sentinel(primary_key)] — add it to exactly one field
 --> tests/compile_fail/no_primary_key.rs:4:12
  |
4 | pub struct User {
  |            ^^^^
```

`sentinel-macros/tests/compile_fail/duplicate_primary_key.rs`:
```rust
use sentinel_core::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key)]
    pub id: i64,
    #[sentinel(primary_key)]
    pub uuid: String,
}

fn main() {}
```

`sentinel-macros/tests/compile_fail/duplicate_primary_key.stderr`:
```
error: only one field can be marked #[sentinel(primary_key)]
 --> tests/compile_fail/duplicate_primary_key.rs:9:9
  |
9 |     pub uuid: String,
  |         ^^^^
```

`sentinel-macros/tests/compile_fail/unknown_attribute.rs`:
```rust
use sentinel_core::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primay_key)]
    pub id: i64,
}

fn main() {}
```

**Step 3: Create the trybuild test runner**

`sentinel-macros/tests/error_test.rs`:
```rust
#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/no_primary_key.rs");
    t.compile_fail("tests/compile_fail/duplicate_primary_key.rs");
    t.compile_fail("tests/compile_fail/unknown_attribute.rs");
}
```

**Step 4: Run tests**

```bash
cargo test -p sentinel-macros --test error_test
```

Expected: trybuild tests PASS (error output matches .stderr files).

Note: On first run, if .stderr files don't match exactly, run with `TRYBUILD=overwrite` to capture actual output, then verify the messages are helpful and adjust .stderr accordingly.

**Step 5: Commit**

```bash
git add Cargo.toml sentinel-macros/Cargo.toml sentinel-macros/tests/
git commit -m "test(macros): add compile-fail tests for error message quality"
```

---

## Task 9: Full Suite Verification + Clippy + Fmt

**Files:**
- Possibly fix any clippy/fmt issues across the workspace

**Step 1: Run full test suite**

```bash
cargo test --workspace
```

Expected: All tests PASS (Phase 1 tests + Phase 2 tests).

**Step 2: Run clippy**

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Fix any warnings.

**Step 3: Check formatting**

```bash
cargo fmt --all -- --check
```

Fix any issues with `cargo fmt --all`.

**Step 4: Commit if any fixes needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Add macro dependencies | cargo check |
| 2 | ModelIR — darling parsing | cargo check |
| 3 | Codegen — Model trait impl | 6 tests |
| 4 | Codegen — Column constants | 5 tests |
| 5 | Codegen — NewModel + create() | 4 tests |
| 6 | Table inference + column rename + skip | 6 tests |
| 7 | derive(Partial) | 3 tests |
| 8 | Error message quality (trybuild) | 3 compile-fail tests |
| 9 | Full suite + clippy + fmt | all tests + lint |

**Total: 9 tasks, ~27 new tests + 3 compile-fail tests, 9 commits**

After Phase 2, developers define models with a single `#[derive(Model)]` and get:
- Model trait implementation
- Type-safe column constants with IDE autocomplete
- Auto-generated NewModel struct for inserts
- `create()` method that builds parameterized InsertQuery
- `#[derive(Partial)]` for compile-time validated narrow selects
- Domain-specific error messages (not trait-bound noise)

### Future Phases (separate plans)

- **Phase 3:** Type-state relations — `User<Bare>` vs `User<WithPosts>`, include/batch_load
- **Phase 4:** Connection trait + sentinel-driver integration
- **Phase 5:** Transaction system with deadlock prevention + `#[reducer]`
- **Phase 6:** `sentinel-migrate` — schema diff, SQL generation
- **Phase 7:** `sentinel-cli` — CLI commands
