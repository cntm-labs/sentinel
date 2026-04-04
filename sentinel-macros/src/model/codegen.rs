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
