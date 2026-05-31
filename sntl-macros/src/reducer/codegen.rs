use proc_macro_error2::abort;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{FnArg, ItemFn, Pat};

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
    if !attr.is_empty() {
        // Task 6 will parse `isolation = "..."` here. For now reject any attr.
        let s = attr
            .into_iter()
            .next()
            .map(|t| t.span())
            .unwrap_or_else(Span::call_site);
        abort!(
            s,
            "#[sntl::reducer] does not accept args yet (isolation = … lands in Task 6)"
        );
    }

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

    quote! {
        #vis #sig {
            use ::std::time::Instant;
            use ::sntl::driver::Event;

            let __name: &'static str = #fn_name_str;
            #conn_ident.instrumentation().on_event(&Event::ReducerBegin { name: __name });
            let __start = Instant::now();

            #conn_ident.begin().await?;

            // No move on the inner async block — body captures `conn` by reference,
            // and the outer scope still needs it for commit/rollback after .await.
            // Panic safety lands in Task 7 (AssertUnwindSafe + catch_unwind).
            match (async { #body }).await {
                Ok(__r) => {
                    #conn_ident.commit().await?;
                    #conn_ident.instrumentation().on_event(&Event::ReducerCommit {
                        name: __name,
                        duration: __start.elapsed(),
                    });
                    Ok(__r)
                }
                Err(__e) => {
                    let __err = format!("{}", __e);
                    #conn_ident.rollback().await.ok();
                    #conn_ident.instrumentation().on_event(&Event::ReducerRollback {
                        name: __name,
                        error: &__err,
                    });
                    Err(__e)
                }
            }
        }
    }
}
