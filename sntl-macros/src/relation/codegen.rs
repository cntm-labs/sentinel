use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::ir::{RelationIR, RelationKindIR};

pub fn generate_relations(ir: &RelationIR) -> TokenStream {
    let relation_consts = generate_relation_constants(ir);
    let pascal_methods = generate_pascal_find_methods(ir);
    let markers = generate_relation_markers(ir);
    let transitions = generate_include_transitions(ir);
    let typed_includes = generate_typed_include_methods(ir);
    let accessors = generate_relation_accessors(ir);
    let type_aliases = generate_type_aliases(ir);
    let bare_state = generate_bare_state(ir);

    quote! {
        #relation_consts
        #pascal_methods
        #markers
        #transitions
        #typed_includes
        #accessors
        #type_aliases
        #bare_state
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

    quote! {
        #[automatically_derived]
        impl #model {
            /// Start a SELECT query (PascalCase API).
            #[allow(non_snake_case)]
            pub fn Find() -> sntl::core::query::ModelQuery<Self> {
                sntl::core::query::ModelQuery::from_table(<Self as sntl::core::Model>::TABLE)
            }

            /// SELECT by primary key (PascalCase API).
            #[allow(non_snake_case)]
            pub fn FindId(id: impl Into<sntl::core::Value>) -> sntl::core::query::ModelQuery<Self> {
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

// ---------------------------------------------------------------------------
// NEW: Relation markers, transitions, typed includes, accessors, aliases
// ---------------------------------------------------------------------------

/// Generate marker structs: `pub struct UserPosts;`
fn generate_relation_markers(ir: &RelationIR) -> TokenStream {
    let markers: Vec<TokenStream> = ir
        .relations
        .iter()
        .map(|rel| {
            let marker_name = marker_ident(&ir.model_name, &rel.fn_name);
            quote! {
                /// Relation marker type for compile-time Include tracking.
                pub struct #marker_name;
            }
        })
        .collect();

    quote! { #(#markers)* }
}

/// Generate `IncludeTransition` impls — one per relation with generics for other positions.
fn generate_include_transitions(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;
    let n = ir.relations.len();

    let impls: Vec<TokenStream> = ir
        .relations
        .iter()
        .enumerate()
        .map(|(i, rel)| {
            let marker = marker_ident(model, &rel.fn_name);

            // Generic params for all positions except i
            let generic_params: Vec<TokenStream> = (0..n)
                .filter(|&j| j != i)
                .map(|j| {
                    let param = format_ident!("_S{}", j);
                    quote! { #param }
                })
                .collect();

            // Current state tuple: generic for all, Unloaded at position i
            let current_tuple: Vec<TokenStream> = (0..n)
                .map(|j| {
                    if j == i {
                        quote! { sntl::core::relation::Unloaded }
                    } else {
                        let param = format_ident!("_S{}", j);
                        quote! { #param }
                    }
                })
                .collect();

            // Next state tuple: generic for all, Loaded at position i
            let next_tuple: Vec<TokenStream> = (0..n)
                .map(|j| {
                    if j == i {
                        quote! { sntl::core::relation::Loaded }
                    } else {
                        let param = format_ident!("_S{}", j);
                        quote! { #param }
                    }
                })
                .collect();

            quote! {
                #[automatically_derived]
                impl<#(#generic_params),*> sntl::core::relation::IncludeTransition<
                    #model, (#(#current_tuple,)*), #marker
                > for () {
                    type Next = (#(#next_tuple,)*);
                }
            }
        })
        .collect();

    quote! { #(#impls)* }
}

/// Generate typed Include methods: `User::Posts() -> RelationInclude<User, UserPosts>`
fn generate_typed_include_methods(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;

    let methods: Vec<TokenStream> = ir
        .relations
        .iter()
        .map(|rel| {
            let pascal_name = to_pascal_case(&rel.fn_name.to_string());
            let method_name = format_ident!("{}", pascal_name);
            let marker = marker_ident(model, &rel.fn_name);
            let fk = &rel.foreign_key;
            let target_table = infer_table_name(&rel.target_type);
            let rel_name = rel.fn_name.to_string();
            let kind_token = match rel.kind {
                RelationKindIR::HasMany => quote!(sntl::core::relation::RelationKind::HasMany),
                RelationKindIR::HasOne => quote!(sntl::core::relation::RelationKind::HasOne),
                RelationKindIR::BelongsTo => quote!(sntl::core::relation::RelationKind::BelongsTo),
            };

            quote! {
                /// Returns a typed relation include for use with `.Include()`.
                #[allow(non_snake_case)]
                pub fn #method_name() -> sntl::core::relation::RelationInclude<#model, #marker> {
                    sntl::core::relation::RelationInclude::new(
                        sntl::core::relation::RelationSpec::new(
                            #rel_name, #fk, #target_table, #kind_token,
                        )
                    )
                }
            }
        })
        .collect();

    quote! {
        #[automatically_derived]
        impl #model {
            #(#methods)*
        }
    }
}

/// Generate accessor trait and RelationLoaded impls for loaded relations.
///
/// Generates a trait `<Model>Relations` with accessor methods (e.g., `posts()`),
/// then implements it for `WithRelations<Model, State>` where the relevant
/// position is `Loaded`. Also implements `RelationLoaded<Marker>` for trait-based access.
fn generate_relation_accessors(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;
    let n = ir.relations.len();
    let trait_name = format_ident!("{}RelAccessors", model);

    // Trait methods — one per relation
    let trait_methods: Vec<TokenStream> = ir
        .relations
        .iter()
        .map(|rel| {
            let accessor_name = &rel.fn_name;
            let target_type = &rel.target_type;
            let return_type = match rel.kind {
                RelationKindIR::HasMany => quote! { &[#target_type] },
                RelationKindIR::HasOne | RelationKindIR::BelongsTo => quote! { &#target_type },
            };
            quote! {
                fn #accessor_name(&self) -> #return_type;
            }
        })
        .collect();

    // Impl per relation — only when that relation is Loaded
    let impls: Vec<TokenStream> = ir
        .relations
        .iter()
        .enumerate()
        .map(|(i, rel)| {
            let marker = marker_ident(model, &rel.fn_name);
            let accessor_name = &rel.fn_name;
            let target_type = &rel.target_type;
            let rel_name = rel.fn_name.to_string();

            // Generic params for all positions except i
            let generic_params: Vec<TokenStream> = (0..n)
                .filter(|&j| j != i)
                .map(|j| {
                    let param = format_ident!("_S{}", j);
                    quote! { #param }
                })
                .collect();

            // State tuple with Loaded at position i, generic elsewhere
            let state_tuple: Vec<TokenStream> = (0..n)
                .map(|j| {
                    if j == i {
                        quote! { sntl::core::relation::Loaded }
                    } else {
                        let param = format_ident!("_S{}", j);
                        quote! { #param }
                    }
                })
                .collect();

            let (return_type, accessor_body) = match rel.kind {
                RelationKindIR::HasMany => (
                    quote! { &[#target_type] },
                    quote! {
                        self.relations()
                            .get::<Vec<#target_type>>(#rel_name)
                            .map(|v| v.as_slice())
                            .unwrap_or(&[])
                    },
                ),
                RelationKindIR::HasOne | RelationKindIR::BelongsTo => (
                    quote! { &#target_type },
                    quote! {
                        self.relations()
                            .get::<#target_type>(#rel_name)
                            .expect(concat!("relation '", #rel_name, "' was loaded but data missing"))
                    },
                ),
            };

            let output_type = match rel.kind {
                RelationKindIR::HasMany => quote! { [#target_type] },
                RelationKindIR::HasOne | RelationKindIR::BelongsTo => quote! { #target_type },
            };

            // Generate both the accessor trait impl and the RelationLoaded trait impl
            quote! {
                #[automatically_derived]
                impl<#(#generic_params),*> #trait_name
                    for sntl::core::relation::WithRelations<#model, (#(#state_tuple,)*)>
                {
                    fn #accessor_name(&self) -> #return_type {
                        #accessor_body
                    }
                }

                #[automatically_derived]
                impl<#(#generic_params),*> sntl::core::relation::RelationLoaded<#marker>
                    for sntl::core::relation::WithRelations<#model, (#(#state_tuple,)*)>
                {
                    type Output = #output_type;
                    fn get_relation(&self) -> &#output_type {
                        <Self as #trait_name>::#accessor_name(self)
                    }
                }
            }
        })
        .collect();

    quote! {
        /// Extension trait for accessing loaded relations on this model.
        pub trait #trait_name {
            #(#trait_methods)*
        }

        #(#impls)*
    }
}

/// Generate type aliases: `UserBare`, `UserWithPosts`, `UserFull`
fn generate_type_aliases(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;
    let n = ir.relations.len();

    let mut aliases = Vec::new();

    // Bare alias — all Unloaded
    let bare_name = format_ident!("{}Bare", model);
    let all_unloaded: Vec<TokenStream> = (0..n)
        .map(|_| quote! { sntl::core::relation::Unloaded })
        .collect();
    aliases.push(quote! {
        /// Type alias: model with no relations loaded.
        pub type #bare_name = sntl::core::relation::WithRelations<#model, (#(#all_unloaded,)*)>;
    });

    // Per-relation aliases — one relation loaded
    for (i, rel) in ir.relations.iter().enumerate() {
        let pascal = to_pascal_case(&rel.fn_name.to_string());
        let alias_name = format_ident!("{}With{}", model, pascal);
        let state: Vec<TokenStream> = (0..n)
            .map(|j| {
                if j == i {
                    quote! { sntl::core::relation::Loaded }
                } else {
                    quote! { sntl::core::relation::Unloaded }
                }
            })
            .collect();
        aliases.push(quote! {
            /// Type alias: model with one relation loaded.
            pub type #alias_name = sntl::core::relation::WithRelations<#model, (#(#state,)*)>;
        });
    }

    // Full alias — all Loaded (only if >0 relations)
    if n > 0 {
        let full_name = format_ident!("{}Full", model);
        let all_loaded: Vec<TokenStream> = (0..n)
            .map(|_| quote! { sntl::core::relation::Loaded })
            .collect();
        aliases.push(quote! {
            /// Type alias: model with all relations loaded.
            pub type #full_name = sntl::core::relation::WithRelations<#model, (#(#all_loaded,)*)>;
        });
    }

    quote! { #(#aliases)* }
}

/// Generate `ModelRelations` impl: `impl ModelRelations for User { type BareState = (Unloaded,); }`
fn generate_bare_state(ir: &RelationIR) -> TokenStream {
    let model = &ir.model_name;
    let n = ir.relations.len();

    let all_unloaded: Vec<TokenStream> = (0..n)
        .map(|_| quote! { sntl::core::relation::Unloaded })
        .collect();

    quote! {
        #[automatically_derived]
        impl sntl::core::relation::ModelRelations for #model {
            type BareState = (#(#all_unloaded,)*);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build marker ident: Model + PascalCase(fn_name) → "UserPosts"
fn marker_ident(model: &syn::Ident, fn_name: &syn::Ident) -> syn::Ident {
    let pascal = to_pascal_case(&fn_name.to_string());
    format_ident!("{}{}", model, pascal)
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
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
