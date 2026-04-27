//! `sntl::query_file!("queries/foo.sql", params…)` and `query_file_as!`.
//!
//! Reads the SQL file relative to `CARGO_MANIFEST_DIR` at proc-macro time
//! and forwards to `query!` / `query_as!` via re-expansion, so the same
//! cache-validation path runs.

use proc_macro_error2::abort;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::path::PathBuf;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitStr, Path, Token};

pub struct QueryFileArgs {
    pub file: LitStr,
    pub params: Vec<Expr>,
    pub overrides_nullable: Vec<Ident>,
    pub overrides_non_null: Vec<Ident>,
}

impl Parse for QueryFileArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let file: LitStr = input.parse()?;
        let mut params = Vec::new();
        let mut overrides_nullable = Vec::new();
        let mut overrides_non_null = Vec::new();
        while input.parse::<Token![,]>().is_ok() {
            if input.is_empty() {
                break;
            }
            if input.peek(Ident) && input.peek2(Token![=]) {
                let key: Ident = input.fork().parse()?;
                if key == "nullable" {
                    let _: Ident = input.parse()?;
                    input.parse::<Token![=]>()?;
                    let content;
                    syn::bracketed!(content in input);
                    overrides_nullable =
                        syn::punctuated::Punctuated::<Ident, Token![,]>::parse_terminated(
                            &content,
                        )?
                        .into_iter()
                        .collect();
                    continue;
                }
                if key == "non_null" {
                    let _: Ident = input.parse()?;
                    input.parse::<Token![=]>()?;
                    let content;
                    syn::bracketed!(content in input);
                    overrides_non_null =
                        syn::punctuated::Punctuated::<Ident, Token![,]>::parse_terminated(
                            &content,
                        )?
                        .into_iter()
                        .collect();
                    continue;
                }
            }
            params.push(input.parse::<Expr>()?);
        }
        Ok(Self {
            file,
            params,
            overrides_nullable,
            overrides_non_null,
        })
    }
}

pub struct QueryFileAsArgs {
    pub target: Path,
    pub inner: QueryFileArgs,
}

impl Parse for QueryFileAsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let target: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let inner = input.parse()?;
        Ok(Self { target, inner })
    }
}

fn load_sql_from(file: &LitStr) -> String {
    let rel = file.value();
    let base = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let path = base.join(&rel);
    match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => abort!(
            file.span(),
            "cannot read SQL file {}: {}",
            path.display(),
            e
        ),
    }
}

pub fn expand(ts: TokenStream) -> TokenStream {
    let span = Span::call_site();
    let args: QueryFileArgs = match syn::parse2(ts) {
        Ok(a) => a,
        Err(e) => abort!(span, "{}", e),
    };
    let sql = load_sql_from(&args.file);
    let lit_sql = LitStr::new(&sql, args.file.span());
    let params = args.params;
    let nullable = args.overrides_nullable;
    let non_null = args.overrides_non_null;
    quote! {
        ::sntl::query!(
            #lit_sql,
            #(#params,)*
            nullable = [#(#nullable),*],
            non_null = [#(#non_null),*]
        )
    }
}

pub fn expand_as(ts: TokenStream) -> TokenStream {
    let span = Span::call_site();
    let args: QueryFileAsArgs = match syn::parse2(ts) {
        Ok(a) => a,
        Err(e) => abort!(span, "{}", e),
    };
    let sql = load_sql_from(&args.inner.file);
    let lit_sql = LitStr::new(&sql, args.inner.file.span());
    let params = args.inner.params;
    let nullable = args.inner.overrides_nullable;
    let non_null = args.inner.overrides_non_null;
    let target = args.target;
    quote! {
        ::sntl::query_as!(
            #target,
            #lit_sql,
            #(#params,)*
            nullable = [#(#nullable),*],
            non_null = [#(#non_null),*]
        )
    }
}
