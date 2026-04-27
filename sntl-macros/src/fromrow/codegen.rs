use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields, FieldsNamed};

pub fn expand(input: DeriveInput) -> TokenStream {
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => named,
        _ => {
            return syn::Error::new_spanned(
                &input.ident,
                "`FromRow` can only derive on structs with named fields",
            )
            .to_compile_error();
        }
    };

    let getters = fields.iter().map(|f| {
        let name = f.ident.as_ref().expect("named field");
        let name_str = name.to_string();
        let ty = &f.ty;
        quote! {
            #name: row.try_get_by_name::<#ty>(#name_str)
                .map_err(|e| ::sntl::Error::Driver(e))?
        }
    });

    quote! {
        impl #impl_generics ::sntl::__macro_support::FromRow for #ident #type_generics #where_clause {
            fn from_row(row: &::sntl::driver::Row) -> ::sntl::Result<Self> {
                Ok(Self {
                    #(#getters),*
                })
            }
        }
    }
}
