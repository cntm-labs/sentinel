//! `sntl::query!("SELECT …", params…)` — anonymous record macro.
//!
//! Each invocation emits a local struct with one field per output column,
//! along with a `FromRow` impl, and returns a `QueryExecution` over it.

use crate::query::args::{idents_to_strings, parse_query_args};
use crate::query::codegen::{CodegenInput, build_handle, build_params, rust_type_for_column};
use crate::query::lookup::{load_schema, lookup_entry};
use proc_macro_error2::abort;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use sntl_schema::resolve::{ResolveInput, resolve_offline};

pub fn expand(ts: TokenStream) -> TokenStream {
    let span = Span::call_site();
    let args = parse_query_args(ts);
    let sql = args.sql.value();
    let entry = lookup_entry(&sql, span);
    let schema = load_schema(span);

    let nullable = idents_to_strings(&args.overrides_nullable);
    let non_null = idents_to_strings(&args.overrides_non_null);
    let resolved = match resolve_offline(ResolveInput {
        sql: &sql,
        cache_entry: &entry,
        schema: &schema,
        overrides_nullable: &nullable,
        overrides_non_null: &non_null,
        overrides_non_null_elements: &[], // wired up in Task 12
        strict: true,
    }) {
        Ok(r) => r,
        Err(e) => abort!(span, "{}", e),
    };

    let struct_ident = format_ident!("__sntl_query_record_{}", entry.sql_hash);

    let field_defs = resolved.columns.iter().map(|c| {
        let name = format_ident!("{}", c.name);
        let ty = rust_type_for_column(c);
        quote! { pub #name: #ty }
    });
    let field_getters = resolved.columns.iter().map(|c| {
        let name = format_ident!("{}", c.name);
        let name_str = &c.name;
        let ty = rust_type_for_column(c);
        quote! {
            #name: row.try_get_by_name::<#ty>(#name_str)
                .map_err(|e| ::sntl::Error::Driver(e))?
        }
    });

    let codegen_input = CodegenInput {
        sql: &sql,
        params: &resolved.params,
        param_exprs: &args.params,
    };
    let handle = build_handle(&codegen_input);
    let params_slice = build_params(&codegen_input);

    quote! {
        {
            #[allow(non_camel_case_types)]
            pub struct #struct_ident {
                #(#field_defs,)*
            }
            impl ::sntl::__macro_support::FromRow for #struct_ident {
                fn from_row(row: &::sntl::driver::Row) -> ::sntl::Result<Self> {
                    Ok(Self { #(#field_getters,)* })
                }
            }
            ::sntl::__macro_support::QueryExecution::<#struct_ident>::new(
                #handle,
                #params_slice,
            )
        }
    }
}
