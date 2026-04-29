//! Argument parsing for `sntl::query!` family macros.
//!
//! Grammar:
//! ```text
//! query!(
//!     "SQL", expr, expr, …
//!     [, nullable = [a, b]]
//!     [, non_null = [c]]
//!     [, non_null_elements = [tags]]
//! )
//! query_as!(Path, "SQL", expr, …)
//! ```

use proc_macro_error2::abort;
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, Ident, LitStr, Path, Token};

pub struct QueryArgs {
    pub sql: LitStr,
    pub params: Vec<Expr>,
    pub overrides_nullable: Vec<Ident>,
    pub overrides_non_null: Vec<Ident>,
    pub overrides_non_null_elements: Vec<Ident>,
}

pub struct QueryAsArgs {
    pub target: Path,
    pub query: QueryArgs,
}

impl Parse for QueryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let sql: LitStr = input.parse()?;
        let mut params = Vec::new();
        let mut overrides_nullable = Vec::new();
        let mut overrides_non_null = Vec::new();
        let mut overrides_non_null_elements = Vec::new();
        while input.parse::<Token![,]>().is_ok() {
            if input.is_empty() {
                break;
            }
            // Look for `ident = …` overrides without consuming on miss.
            if input.peek(Ident) && input.peek2(Token![=]) {
                let key: Ident = input.fork().parse()?;
                match key.to_string().as_str() {
                    "nullable" => {
                        let _key: Ident = input.parse()?;
                        input.parse::<Token![=]>()?;
                        overrides_nullable = parse_ident_list(input)?.into_iter().collect();
                        continue;
                    }
                    "non_null" => {
                        let _key: Ident = input.parse()?;
                        input.parse::<Token![=]>()?;
                        overrides_non_null = parse_ident_list(input)?.into_iter().collect();
                        continue;
                    }
                    "non_null_elements" => {
                        let _key: Ident = input.parse()?;
                        input.parse::<Token![=]>()?;
                        overrides_non_null_elements =
                            parse_ident_list(input)?.into_iter().collect();
                        continue;
                    }
                    _ => {}
                }
            }
            params.push(input.parse::<Expr>()?);
        }
        Ok(QueryArgs {
            sql,
            params,
            overrides_nullable,
            overrides_non_null,
            overrides_non_null_elements,
        })
    }
}

fn parse_ident_list(input: ParseStream) -> syn::Result<Punctuated<Ident, Token![,]>> {
    let content;
    syn::bracketed!(content in input);
    Punctuated::<Ident, Token![,]>::parse_terminated(&content)
}

impl Parse for QueryAsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let target: Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let query: QueryArgs = input.parse()?;
        Ok(QueryAsArgs { target, query })
    }
}

/// Convert override identifiers into the `String` shape expected by
/// `sntl_schema::resolve::ResolveInput`.
pub fn idents_to_strings(idents: &[Ident]) -> Vec<String> {
    idents.iter().map(|i| i.to_string()).collect()
}

pub fn parse_query_args(ts: TokenStream) -> QueryArgs {
    match syn::parse2::<QueryArgs>(ts) {
        Ok(a) => a,
        Err(e) => abort!(e.span(), "{}", e),
    }
}

pub fn parse_query_as_args(ts: TokenStream) -> QueryAsArgs {
    match syn::parse2::<QueryAsArgs>(ts) {
        Ok(a) => a,
        Err(e) => abort!(e.span(), "{}", e),
    }
}
