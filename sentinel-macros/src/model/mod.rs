pub mod ir;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;

use ir::ModelOpts;

pub fn derive_model_impl(input: TokenStream) -> TokenStream {
    let derive_input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(di) => di,
        Err(e) => return e.to_compile_error(),
    };

    let opts = match ModelOpts::from_derive_input(&derive_input) {
        Ok(o) => o,
        Err(e) => return e.write_errors(),
    };

    let ir = match opts.into_ir() {
        Ok(ir) => ir,
        Err(e) => return e.write_errors(),
    };

    // Stub: just generate an empty impl to verify parsing works
    let name = &ir.struct_name;
    let table = &ir.table_name;

    quote! {
        impl #name {
            /// Table name (temporary stub — full codegen in next task).
            pub const __TABLE: &'static str = #table;
        }
    }
}
