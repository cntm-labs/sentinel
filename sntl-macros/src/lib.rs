//! Sentinel Macros — derive(Model), derive(Partial), #[sentinel(relations)].

mod model;
mod partial;
mod relation;

use proc_macro::TokenStream;

/// Derive the `Model` trait for a struct.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Model)]
/// #[sentinel(table = "users")]
/// pub struct User {
///     #[sentinel(primary_key, default = "gen_random_uuid()")]
///     pub id: Uuid,
///     pub email: String,
/// }
/// ```
#[proc_macro_derive(Model, attributes(sentinel))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    model::derive_model_impl(input.into()).into()
}

/// Derive a partial select type.
#[proc_macro_derive(Partial, attributes(sentinel))]
pub fn derive_partial(input: TokenStream) -> TokenStream {
    partial::derive_partial_impl(input.into()).into()
}

/// Declare relations on a model.
///
/// ```rust,ignore
/// #[sentinel(relations)]
/// impl User {
///     pub fn posts() -> HasMany<Post> { HasMany::new("user_id") }
/// }
/// ```
#[proc_macro_attribute]
pub fn sentinel(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_str = attr.to_string();
    if attr_str.trim() == "relations" {
        relation::expand_relations(item.into()).into()
    } else {
        panic!("unknown sentinel attribute: `{attr_str}` — expected `relations`");
    }
}
