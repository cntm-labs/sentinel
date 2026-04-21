//! `sntl::query_as!(Target, "SELECT …", params…)`
//! and `sntl::query_scalar!("SELECT one_col FROM …", params…)`.

use crate::query::args::{idents_to_strings, parse_query_args, parse_query_as_args};
use crate::query::codegen::{build_handle, build_params, rust_type_for_column, CodegenInput};
use crate::query::lookup::{load_schema, lookup_entry};
use proc_macro2::{Span, TokenStream};
use proc_macro_error2::abort;
use quote::quote;
use sntl_schema::resolve::{resolve_offline, ResolveInput};

pub fn expand_as(ts: TokenStream) -> TokenStream {
    let span = Span::call_site();
    let args = parse_query_as_args(ts);
    let sql = args.query.sql.value();
    let entry = lookup_entry(&sql, span);
    let schema = load_schema(span);

    let nullable = idents_to_strings(&args.query.overrides_nullable);
    let non_null = idents_to_strings(&args.query.overrides_non_null);
    let resolved = match resolve_offline(ResolveInput {
        sql: &sql,
        cache_entry: &entry,
        schema: &schema,
        overrides_nullable: &nullable,
        overrides_non_null: &non_null,
        strict: true,
    }) {
        Ok(r) => r,
        Err(e) => abort!(span, "{}", e),
    };

    let target = &args.target;
    let codegen = CodegenInput {
        sql: &sql,
        params: &resolved.params,
        param_exprs: &args.query.params,
    };
    let handle = build_handle(&codegen);
    let params_slice = build_params(&codegen);

    quote! {
        {
            // Compile-time bound check that the user's target type implements
            // FromRow — surfaces a clear error instead of failing inside the
            // QueryExecution generic instantiation.
            fn _assert_from_row<T: ::sntl::__macro_support::FromRow>() {}
            _assert_from_row::<#target>();
            ::sntl::__macro_support::QueryExecution::<#target>::new(
                #handle,
                #params_slice,
            )
        }
    }
}

pub fn expand_scalar(ts: TokenStream) -> TokenStream {
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
        strict: true,
    }) {
        Ok(r) => r,
        Err(e) => abort!(span, "{}", e),
    };

    if resolved.columns.len() != 1 {
        abort!(
            span,
            "query_scalar! expects exactly one output column, got {}",
            resolved.columns.len()
        );
    }
    let col = &resolved.columns[0];
    let ty = rust_type_for_column(col);
    let col_name = &col.name;

    let codegen = CodegenInput {
        sql: &sql,
        params: &resolved.params,
        param_exprs: &args.params,
    };
    let handle = build_handle(&codegen);
    let params_slice = build_params(&codegen);

    quote! {
        {
            // Local wrapper struct so we can re-use QueryExecution + FromRow
            // for a single scalar column. The wrapper is unwrapped by the
            // ScalarExecution extract closure.
            #[allow(non_camel_case_types)]
            struct __SntlScalar(pub #ty);
            impl ::sntl::__macro_support::FromRow for __SntlScalar {
                fn from_row(row: &::sntl::driver::Row) -> ::sntl::Result<Self> {
                    Ok(Self(
                        row.try_get_by_name::<#ty>(#col_name)
                            .map_err(|e| ::sntl::Error::Driver(e))?,
                    ))
                }
            }
            ::sntl::__macro_support::ScalarExecution::<#ty, __SntlScalar>::new(
                ::sntl::__macro_support::QueryExecution::<__SntlScalar>::new(
                    #handle,
                    #params_slice,
                ),
                |w| w.0,
            )
        }
    }
}
