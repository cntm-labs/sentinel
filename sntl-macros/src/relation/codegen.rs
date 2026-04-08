use proc_macro2::TokenStream;
use quote::quote;

use super::ir::{RelationIR, RelationKindIR};

pub fn generate_relations(ir: &RelationIR) -> TokenStream {
    let relation_consts = generate_relation_constants(ir);
    let pascal_methods = generate_pascal_find_methods(ir);

    quote! {
        #relation_consts
        #pascal_methods
    }
}

fn generate_relation_constants(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;
    let consts: Vec<TokenStream> = ir
        .relations
        .iter()
        .map(|rel| {
            let const_name = &rel.const_name;
            let fk = &rel.foreign_key;
            let target_table = infer_table_name(&rel.target_type);
            let rel_name = rel.fn_name.to_string();
            let kind_token = match rel.kind {
                RelationKindIR::HasMany => {
                    quote!(sntl::core::relation::RelationKind::HasMany)
                }
                RelationKindIR::HasOne => {
                    quote!(sntl::core::relation::RelationKind::HasOne)
                }
                RelationKindIR::BelongsTo => {
                    quote!(sntl::core::relation::RelationKind::BelongsTo)
                }
            };
            quote! {
                #[allow(non_upper_case_globals)]
                pub const #const_name: sntl::core::relation::RelationSpec =
                    sntl::core::relation::RelationSpec::new_const(
                        #rel_name,
                        #fk,
                        #target_table,
                        #kind_token,
                    );
            }
        })
        .collect();

    quote! {
        #[automatically_derived]
        impl #model {
            #(#consts)*
        }
    }
}

fn generate_pascal_find_methods(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;

    // Uses Self::TABLE from the Model trait — works with any table name,
    // including custom #[sentinel(table = "...")] overrides.
    quote! {
        #[automatically_derived]
        impl #model {
            /// Start a SELECT query (PascalCase API).
            #[allow(non_snake_case)]
            pub fn Find() -> sntl::core::query::ModelQuery {
                sntl::core::query::ModelQuery::from_table(<Self as sntl::core::Model>::TABLE)
            }

            /// SELECT by primary key (PascalCase API).
            #[allow(non_snake_case)]
            pub fn FindId(id: impl Into<sntl::core::Value>) -> sntl::core::query::ModelQuery {
                let pk_col = sntl::core::Column {
                    table: std::borrow::Cow::Borrowed(<Self as sntl::core::Model>::TABLE),
                    name: std::borrow::Cow::Borrowed(<Self as sntl::core::Model>::PRIMARY_KEY),
                };
                sntl::core::query::ModelQuery::from_table(<Self as sntl::core::Model>::TABLE)
                    .Where(pk_col.eq(id))
            }
        }
    }
}

fn infer_table_name(ty: &syn::Type) -> String {
    if let syn::Type::Path(tp) = ty
        && let Some(seg) = tp.path.segments.last()
    {
        let name = seg.ident.to_string();
        return format!("{}s", to_snake_case(&name));
    }
    "unknown".to_string()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}
