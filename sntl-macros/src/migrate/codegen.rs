use std::path::PathBuf;

use proc_macro_error2::abort;
use proc_macro2::TokenStream;
use quote::quote;
use syn::LitStr;

/// Expand `sntl_migrate::migrate!("./migrations")` into
/// `::sntl_migrate::Migrator::from_static(&[...])` populated at compile time.
pub fn expand(input: TokenStream) -> TokenStream {
    let lit: LitStr = match syn::parse2(input) {
        Ok(l) => l,
        Err(e) => abort!(e.span(), "{}", e),
    };
    let rel = lit.value();
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let migrations_dir = manifest.join(&rel);
    if !migrations_dir.is_dir() {
        abort!(
            lit.span(),
            "migrations directory not found: {}",
            migrations_dir.display()
        );
    }

    let read = match std::fs::read_dir(&migrations_dir) {
        Ok(r) => r,
        Err(e) => abort!(lit.span(), "read_dir({}): {e}", migrations_dir.display()),
    };

    let mut entries: Vec<(String, PathBuf, bool)> = Vec::new();
    for entry in read.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let dir = entry.path();
        let (sql_path, is_notx) = if dir.join("up.notx.sql").exists() {
            (dir.join("up.notx.sql"), true)
        } else if dir.join("up.sql").exists() {
            (dir.join("up.sql"), false)
        } else {
            abort!(
                lit.span(),
                "migration `{}` has neither up.sql nor up.notx.sql",
                name
            );
        };
        entries.push((name, sql_path, is_notx));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let tokens: Vec<TokenStream> = entries
        .iter()
        .map(|(name, path, is_notx)| {
            let path_str = path.to_string_lossy().into_owned();
            let mode = if *is_notx {
                quote! { ::sntl_migrate::TxMode::None }
            } else {
                quote! { ::sntl_migrate::TxMode::PerMigration }
            };
            quote! {
                (#name, include_str!(#path_str), #mode),
            }
        })
        .collect();

    quote! {
        ::sntl_migrate::Migrator::from_static(&[
            #(#tokens)*
        ])
    }
}
