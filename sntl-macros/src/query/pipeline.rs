//! `sntl::query_pipeline!(conn, name1: "SQL", name2: "SQL" using Target, …)`.
//!
//! Sends every named query in one PipelineBatch (single round-trip) and
//! returns the driver's `Vec<QueryResult>`. Each query's parameter OIDs
//! come from the `.sentinel/` cache; parameters are encoded eagerly with
//! `encode_params` so the runtime layer is just a thin batch-builder.

use crate::query::lookup::{load_schema, lookup_entry};
use proc_macro_error2::abort;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use sntl_schema::resolve::{ResolveInput, resolve_offline};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitStr, Path, Token};

struct PipelineEntry {
    _name: Ident,
    sql: LitStr,
    _target: Option<Path>,
    params: Vec<Expr>,
}

struct PipelineArgs {
    conn: Expr,
    entries: Vec<PipelineEntry>,
}

impl Parse for PipelineArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let conn: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let mut entries = vec![];
        while !input.is_empty() {
            // Each entry: `name: "SQL" [using Target] [, params…]`
            // followed by `;` separator (or end of input).
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let sql: LitStr = input.parse()?;

            let target = if input.peek(Ident)
                && input
                    .fork()
                    .parse::<Ident>()
                    .map(|i| i == "using")
                    .unwrap_or(false)
            {
                let _: Ident = input.parse()?;
                Some(input.parse::<Path>()?)
            } else {
                None
            };

            // Optional positional params after a comma — terminate at `;` or EOF
            // or when the next entry name is detected.
            let mut params = vec![];
            while input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                if input.is_empty() || input.peek(Token![;]) {
                    break;
                }
                if input.peek(Ident) && input.peek2(Token![:]) {
                    break;
                }
                params.push(input.parse::<Expr>()?);
            }

            entries.push(PipelineEntry {
                _name: name,
                sql,
                _target: target,
                params,
            });

            // Allow optional `;` between entries.
            let _ = input.parse::<Token![;]>();
        }
        Ok(PipelineArgs { conn, entries })
    }
}

pub fn expand(ts: TokenStream) -> TokenStream {
    let span = Span::call_site();
    let args: PipelineArgs = match syn::parse2(ts) {
        Ok(a) => a,
        Err(e) => abort!(span, "{}", e),
    };
    let schema = load_schema(span);

    let mut spec_items = Vec::with_capacity(args.entries.len());
    for e in &args.entries {
        let sql = e.sql.value();
        let entry = lookup_entry(&sql, e.sql.span());
        let resolved = match resolve_offline(ResolveInput {
            sql: &sql,
            cache_entry: &entry,
            schema: &schema,
            overrides_nullable: &[],
            overrides_non_null: &[],
            overrides_non_null_elements: &[],
            strict: true,
        }) {
            Ok(r) => r,
            Err(err) => abort!(e.sql.span(), "{}", err),
        };

        let lit_sql = LitStr::new(&sql, e.sql.span());
        let oids = resolved.params.iter().map(|p| p.oid);
        let params = &e.params;
        spec_items.push(quote! {
            ::sntl::__macro_support::PipelineQuerySpec {
                sql: #lit_sql,
                param_oids: ::std::vec![ #( ::sntl::Oid::from(#oids) ),* ],
                encoded_params: ::sntl::__macro_support::encode_params(
                    &[ #( &(#params) as &(dyn ::sntl::driver::ToSql + ::std::marker::Sync) ),* ]
                )?,
            }
        });
    }

    let conn = &args.conn;
    quote! {
        {
            let __specs: ::std::vec::Vec<::sntl::__macro_support::PipelineQuerySpec<'_>> =
                ::std::vec![ #( #spec_items ),* ];
            ::sntl::__macro_support::PipelineExecution::new(__specs).run(#conn)
        }
    }
}
