//! `sntl::query_unchecked!("SQL", params…)` and `query_as_unchecked!`.
//!
//! Bypasses `.sentinel/` lookup and runs through the driver's untyped
//! `query`/`execute` path. Use as an escape hatch when you can't or don't
//! want to commit cache state for a particular SQL string.

use proc_macro_error2::abort;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Path, Token};

pub struct UncheckedArgs {
    pub sql: LitStr,
    pub params: Vec<Expr>,
}

impl Parse for UncheckedArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let sql: LitStr = input.parse()?;
        let mut params = vec![];
        while input.parse::<Token![,]>().is_ok() {
            if input.is_empty() {
                break;
            }
            params.push(input.parse()?);
        }
        Ok(Self { sql, params })
    }
}

pub struct UncheckedAsArgs {
    pub target: Path,
    pub inner: UncheckedArgs,
}

impl Parse for UncheckedAsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let target: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let inner = input.parse()?;
        Ok(Self { target, inner })
    }
}

pub fn expand(ts: TokenStream) -> TokenStream {
    let args: UncheckedArgs = match syn::parse2(ts) {
        Ok(a) => a,
        Err(e) => abort!(e.span(), "{}", e),
    };
    let sql = args.sql;
    let params = args.params;
    quote! {
        ::sntl::__macro_support::UncheckedExecution::new(
            #sql,
            ::std::vec![ #( &(#params) as &(dyn ::sntl::driver::ToSql + ::std::marker::Sync) ),* ],
        )
    }
}

pub fn expand_as(ts: TokenStream) -> TokenStream {
    let args: UncheckedAsArgs = match syn::parse2(ts) {
        Ok(a) => a,
        Err(e) => abort!(e.span(), "{}", e),
    };
    let target = args.target;
    let sql = args.inner.sql;
    let params = args.inner.params;
    // TODO(v0.6): query_as_unchecked!(T, ...) currently parses `T` for source
    // compatibility but does not enforce it on the resulting expression because
    // UncheckedExecution dropped its phantom `T` in v0.5 to make
    // `query_unchecked!().into_stream().fetch_stream()` infer cleanly. Restore
    // the T-binding contract by introducing a thin typed wrapper
    // (UncheckedExecutionAs<T>) the macro can return — tracked alongside the
    // v0.6 driver-side query_stream_typed work.
    let _ = target;
    quote! {
        ::sntl::__macro_support::UncheckedExecution::new(
            #sql,
            ::std::vec![ #( &(#params) as &(dyn ::sntl::driver::ToSql + ::std::marker::Sync) ),* ],
        )
    }
}
