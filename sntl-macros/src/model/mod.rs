pub mod codegen;
pub mod ir;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;

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

    let model_impl = codegen::generate_model_impl(&ir);
    let column_consts = codegen::generate_column_consts(&ir);
    let new_struct = codegen::generate_new_struct(&ir);
    let create_method = codegen::generate_create_method(&ir);
    let execution_methods = codegen::generate_execution_methods(&ir);

    quote::quote! {
        #model_impl
        #column_consts
        #new_struct
        #create_method
        #execution_methods
    }
}
