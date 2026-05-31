use proc_macro_error2::abort;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{FnArg, ItemFn, LitStr, Meta, Pat, Token};

/// Parsed attribute args: `isolation = "..."` (optional).
struct Args {
    isolation: Option<IsolationKind>,
}

#[derive(Clone, Copy)]
enum IsolationKind {
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut isolation = None;
        if input.is_empty() {
            return Ok(Self { isolation });
        }
        let metas: Punctuated<Meta, Token![,]> = Punctuated::parse_terminated(input)?;
        for m in metas {
            match &m {
                Meta::NameValue(nv) if nv.path.is_ident("isolation") => {
                    let lit_str = match &nv.value {
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(s),
                            ..
                        }) => s,
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &nv.value,
                                "expected string literal",
                            ));
                        }
                    };
                    isolation = Some(parse_isolation(lit_str)?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &m,
                        "unknown #[sntl::reducer] arg (expected `isolation = \"...\"`)",
                    ));
                }
            }
        }
        Ok(Self { isolation })
    }
}

fn parse_isolation(s: &LitStr) -> syn::Result<IsolationKind> {
    match s.value().as_str() {
        "read_committed" => Ok(IsolationKind::ReadCommitted),
        "repeatable_read" => Ok(IsolationKind::RepeatableRead),
        "serializable" => Ok(IsolationKind::Serializable),
        _ => Err(syn::Error::new_spanned(
            s,
            "isolation must be one of: read_committed, repeatable_read, serializable",
        )),
    }
}

fn type_path_ends_with(ty: &syn::Type, name: &str) -> bool {
    let syn::Type::Path(tp) = ty else {
        return false;
    };
    tp.path
        .segments
        .last()
        .map(|s| s.ident == name)
        .unwrap_or(false)
}

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args: Args = match syn::parse2(attr) {
        Ok(a) => a,
        Err(e) => abort!(e.span(), "{}", e),
    };

    let input_fn: ItemFn = match syn::parse2(item) {
        Ok(f) => f,
        Err(e) => abort!(e.span(), "{}", e),
    };

    if input_fn.sig.asyncness.is_none() {
        abort!(
            input_fn.sig.fn_token.span,
            "#[sntl::reducer] requires async fn"
        );
    }

    // Find the first arg's binding name and validate it is `&mut <...>::Connection`.
    let conn_ident = match input_fn.sig.inputs.iter().next() {
        Some(FnArg::Typed(pat_type)) => {
            let ok = match &*pat_type.ty {
                syn::Type::Reference(r) => {
                    r.mutability.is_some() && type_path_ends_with(&r.elem, "Connection")
                }
                _ => false,
            };
            if !ok {
                abort!(
                    pat_type.ty.span(),
                    "first arg of #[sntl::reducer] must be `&mut driver::Connection`"
                );
            }
            match &*pat_type.pat {
                Pat::Ident(ident) => &ident.ident,
                _ => abort!(
                    pat_type.pat.span(),
                    "first arg of #[sntl::reducer] must be a simple identifier (e.g. `conn: &mut Connection`)"
                ),
            }
        }
        _ => abort!(
            input_fn.sig.fn_token.span,
            "#[sntl::reducer] requires a first arg `conn: &mut driver::Connection`"
        ),
    };

    let fn_name_str = input_fn.sig.ident.to_string();
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let body = &input_fn.block;

    let begin_expr = match args.isolation {
        None => quote! { #conn_ident.begin().await?; },
        Some(IsolationKind::ReadCommitted) => quote! {
            #conn_ident.begin_with(
                ::sntl::driver::TransactionConfig::new()
                    .isolation(::sntl::driver::IsolationLevel::ReadCommitted)
            ).await?;
        },
        Some(IsolationKind::RepeatableRead) => quote! {
            #conn_ident.begin_with(
                ::sntl::driver::TransactionConfig::new()
                    .isolation(::sntl::driver::IsolationLevel::RepeatableRead)
            ).await?;
        },
        Some(IsolationKind::Serializable) => quote! {
            #conn_ident.begin_with(
                ::sntl::driver::TransactionConfig::new()
                    .isolation(::sntl::driver::IsolationLevel::Serializable)
            ).await?;
        },
    };

    quote! {
        #vis #sig {
            use ::std::time::Instant;
            use ::sntl::driver::Event;

            let __name: &'static str = #fn_name_str;
            #conn_ident.instrumentation().on_event(&Event::ReducerBegin { name: __name });
            let __start = Instant::now();

            #begin_expr

            let __conn_ptr: *mut ::sntl::driver::Connection = #conn_ident;
            let __result = ::sntl::__macro_support::FutureExt::catch_unwind(
                ::std::panic::AssertUnwindSafe(async move {
                    // SAFETY: __conn_ptr was derived from #conn_ident whose outer borrow
                    // outlives this scope, and we do not run the outer borrow concurrently
                    // with this future. Re-borrowing here is sound.
                    let #conn_ident = unsafe { &mut *__conn_ptr };
                    #body
                })
            ).await;

            match __result {
                Ok(Ok(__r)) => {
                    #conn_ident.commit().await?;
                    #conn_ident.instrumentation().on_event(&Event::ReducerCommit {
                        name: __name,
                        duration: __start.elapsed(),
                    });
                    Ok(__r)
                }
                Ok(Err(__e)) => {
                    let __err = format!("{}", __e);
                    #conn_ident.rollback().await.ok();
                    #conn_ident.instrumentation().on_event(&Event::ReducerRollback {
                        name: __name,
                        error: &__err,
                    });
                    Err(__e)
                }
                Err(__panic_payload) => {
                    #conn_ident.rollback().await.ok();
                    #conn_ident.instrumentation().on_event(&Event::ReducerRollback {
                        name: __name,
                        error: "panic",
                    });
                    ::std::panic::resume_unwind(__panic_payload);
                }
            }
        }
    }
}
