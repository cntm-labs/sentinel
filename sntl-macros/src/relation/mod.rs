pub mod codegen;
pub mod ir;

use proc_macro2::TokenStream;

pub fn expand_relations(input: TokenStream) -> TokenStream {
    let item_impl = match syn::parse2::<syn::ItemImpl>(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    let ir = match ir::RelationIR::parse(&item_impl) {
        Ok(ir) => ir,
        Err(e) => return e.to_compile_error(),
    };

    codegen::generate_relations(&ir)
}
