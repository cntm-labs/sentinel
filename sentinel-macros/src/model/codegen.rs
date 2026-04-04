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
            pub fn create(new: #new_name) -> sentinel_core::query::InsertQuery {
                sentinel_core::query::InsertQuery::new(#table)
                    #(#column_calls)*
            }
        }
    }
}
