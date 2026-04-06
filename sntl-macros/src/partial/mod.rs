pub mod codegen;
pub mod ir;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;

use ir::PartialOpts;

pub fn derive_partial_impl(input: TokenStream) -> TokenStream {
    let derive_input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(di) => di,
        Err(e) => return e.to_compile_error(),
    };

    let opts = match PartialOpts::from_derive_input(&derive_input) {
        Ok(o) => o,
        Err(e) => return e.write_errors(),
    };

    let ir = match opts.into_ir() {
        Ok(ir) => ir,
        Err(e) => return e.write_errors(),
    };

    codegen::generate_partial_impl(&ir)
}
