mod codegen;

use proc_macro2::TokenStream;
use syn::{parse2, DeriveInput};

pub fn derive_fromrow_impl(input: TokenStream) -> TokenStream {
    let parsed: DeriveInput = match parse2(input) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error(),
    };
    codegen::expand(parsed)
}
