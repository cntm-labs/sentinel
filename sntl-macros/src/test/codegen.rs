use proc_macro_error2::abort;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{ItemFn, LitStr, Meta, Token};

struct Args {
    migrations_dir: Option<String>,
    fixtures: Vec<String>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut migrations_dir = None;
        let mut fixtures = Vec::new();

        if input.is_empty() {
            return Ok(Self {
                migrations_dir,
                fixtures,
            });
        }

        let metas: Punctuated<Meta, Token![,]> = Punctuated::parse_terminated(input)?;
        for m in metas {
            match &m {
                Meta::NameValue(nv) if nv.path.is_ident("migrations") => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        migrations_dir = Some(s.value());
                    } else {
                        return Err(syn::Error::new_spanned(
                            &nv.value,
                            "expected string literal",
                        ));
                    }
                }
                Meta::List(l) if l.path.is_ident("fixtures") => {
                    let names: Punctuated<LitStr, Token![,]> =
                        l.parse_args_with(Punctuated::parse_terminated)?;
                    fixtures = names.into_iter().map(|s| s.value()).collect();
                }
                _ => {
                    return Err(syn::Error::new_spanned(&m, "unknown sntl::test argument"));
                }
            }
        }

        Ok(Self {
            migrations_dir,
            fixtures,
        })
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args: Args = match syn::parse2(attr) {
        Ok(a) => a,
        Err(e) => abort!(e.span(), "{}", e),
    };
    let mut input_fn: ItemFn = match syn::parse2(item) {
        Ok(f) => f,
        Err(e) => abort!(e.span(), "{}", e),
    };

    if input_fn.sig.asyncness.is_none() {
        abort!(
            input_fn.sig.fn_token.span,
            "#[sntl::test] requires async fn"
        );
    }

    let fn_name_str = input_fn.sig.ident.to_string();
    let body = input_fn.block.clone();
    let inputs = input_fn.sig.inputs.clone();

    let migrations_dir_lit = match args.migrations_dir {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };
    let fixtures_lits: Vec<_> = args.fixtures.iter().map(|s| quote! { #s }).collect();

    input_fn.sig.asyncness = None;
    input_fn.block = syn::parse2(quote! {
        {
            let cfg = ::sntl::testing::run::TestConfig {
                test_name: #fn_name_str,
                migrations_dir: #migrations_dir_lit,
                fixtures_dir: None,
                fixtures: &[ #(#fixtures_lits),* ],
            };
            let rt = ::tokio::runtime::Runtime::new().expect("build test tokio runtime");
            rt.block_on(::sntl::testing::run::run(cfg, |pool| async move {
                async fn __body(#inputs) -> ::anyhow::Result<()> {
                    #body
                }
                __body(pool).await
            }));
        }
    })
    .expect("test body parse");

    quote! {
        #[::core::prelude::v1::test]
        #input_fn
    }
}
