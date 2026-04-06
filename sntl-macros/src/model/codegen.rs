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
                sntl::core::ModelColumn {
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
        impl sntl::core::Model for #name {
            const TABLE: &'static str = #table;
            const PRIMARY_KEY: &'static str = #pk_name;

            fn columns() -> &'static [sntl::core::ModelColumn] {
                static COLUMNS: [sntl::core::ModelColumn; #num_columns] = [
                    #(#column_entries),*
                ];
                &COLUMNS
            }
        }
    }
}

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
                pub const #const_name: sntl::core::Column = sntl::core::Column {
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

/// Generate the `New<Model>` struct for INSERT (skips fields with `default`).
pub fn generate_new_struct(ir: &ModelIR) -> TokenStream {
    let new_name = syn::Ident::new(&format!("New{}", ir.struct_name), ir.struct_name.span());

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

/// Generate `from_row()` method that decodes a driver Row into the model struct.
pub fn generate_from_row(ir: &ModelIR) -> TokenStream {
    let name = &ir.struct_name;

    let field_extractions: Vec<TokenStream> = ir
        .fields
        .iter()
        .map(|f| {
            let field_name = &f.field_name;
            if f.skip {
                quote! { #field_name: std::default::Default::default() }
            } else {
                let col_name = &f.column_name;
                quote! { #field_name: row.try_get_by_name(#col_name)? }
            }
        })
        .collect();

    quote! {
        #[automatically_derived]
        impl #name {
            /// Decode a [`sntl::core::Row`] into this model.
            pub fn from_row(row: &sntl::core::Row) -> sntl::driver::Result<Self> {
                Ok(Self {
                    #(#field_extractions,)*
                })
            }
        }
    }
}

/// Generate async execution methods: find_all, find_one, find_optional, create_exec, delete_by_id.
pub fn generate_execution_methods(ir: &ModelIR) -> TokenStream {
    let name = &ir.struct_name;
    let table = &ir.table_name;
    let pk_field = &ir.fields[ir.primary_key_index];
    let pk_name = &pk_field.column_name;
    let new_name = syn::Ident::new(&format!("New{}", ir.struct_name), ir.struct_name.span());

    let insert_column_calls: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.skip && !f.has_default)
        .map(|f| {
            let col_name = &f.column_name;
            let field_name = &f.field_name;
            quote! { .column(#col_name, new.#field_name) }
        })
        .collect();

    let select_sql = format!("SELECT \"{table}\".* FROM \"{table}\"");
    let select_by_id_sql =
        format!("SELECT \"{table}\".* FROM \"{table}\" WHERE \"{pk_name}\" = $1");
    let delete_by_id_sql = format!("DELETE FROM \"{table}\" WHERE \"{pk_name}\" = $1");

    quote! {
        #[automatically_derived]
        impl #name {
            /// Fetch all rows from this model's table.
            pub async fn find_all(
                conn: &mut sntl::core::Connection,
            ) -> sntl::core::Result<Vec<Self>> {
                let rows = conn.query(#select_sql, &[]).await?;
                rows.into_iter()
                    .map(|r| Self::from_row(&r).map_err(sntl::core::Error::from))
                    .collect()
            }

            /// Fetch one row by primary key. Returns error if not found.
            pub async fn find_one(
                conn: &mut sntl::core::Connection,
                id: &(dyn sntl::core::ToSql + Sync),
            ) -> sntl::core::Result<Self> {
                let row = conn.query_one(#select_by_id_sql, &[id]).await?;
                Self::from_row(&row).map_err(sntl::core::Error::from)
            }

            /// Fetch one row by primary key. Returns None if not found.
            pub async fn find_optional(
                conn: &mut sntl::core::Connection,
                id: &(dyn sntl::core::ToSql + Sync),
            ) -> sntl::core::Result<Option<Self>> {
                match conn.query_opt(#select_by_id_sql, &[id]).await? {
                    Some(row) => Ok(Some(
                        Self::from_row(&row).map_err(sntl::core::Error::from)?,
                    )),
                    None => Ok(None),
                }
            }

            /// Insert a new row and return the created model (via RETURNING *).
            pub async fn create_exec(
                conn: &mut sntl::core::Connection,
                new: #new_name,
            ) -> sntl::core::Result<Self> {
                let q = sntl::core::InsertQuery::new(#table)
                    #(#insert_column_calls)*;
                let rows = q.fetch_returning(conn).await?;
                let row = rows
                    .into_iter()
                    .next()
                    .ok_or(sntl::core::Error::NotFound)?;
                Self::from_row(&row).map_err(sntl::core::Error::from)
            }

            /// Delete a row by primary key. Returns the number of rows deleted.
            pub async fn delete_by_id(
                conn: &mut sntl::core::Connection,
                id: &(dyn sntl::core::ToSql + Sync),
            ) -> sntl::core::Result<u64> {
                Ok(conn.execute(#delete_by_id_sql, &[id]).await?)
            }
        }
    }
}

/// Generate the `create(new) -> InsertQuery` method.
pub fn generate_create_method(ir: &ModelIR) -> TokenStream {
    let struct_name = &ir.struct_name;
    let new_name = syn::Ident::new(&format!("New{}", ir.struct_name), ir.struct_name.span());
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
            pub fn create(new: #new_name) -> sntl::core::InsertQuery {
                sntl::core::InsertQuery::new(#table)
                    #(#column_calls)*
            }
        }
    }
}
