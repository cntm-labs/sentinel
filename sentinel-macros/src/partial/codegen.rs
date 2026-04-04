use proc_macro2::TokenStream;
use quote::quote;

use super::ir::PartialIR;

pub fn generate_partial_impl(ir: &PartialIR) -> TokenStream {
    let struct_name = &ir.struct_name;
    let model_ident = syn::Ident::new(&ir.model_name, struct_name.span());

    let column_strs: Vec<&str> = ir.fields.iter().map(|f| f.column_name.as_str()).collect();

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
