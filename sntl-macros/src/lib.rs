//! Sentinel Macros — derive(Model), derive(Partial), #[sentinel(relations)],
//! derive(FromRow), and the sntl::query!() family.

mod fromrow;
mod model;
mod partial;
mod query;
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

/// Derive `FromRow` for ad-hoc structs returned by `sntl::query_as!`.
///
/// Each named field is filled in by `row.try_get_by_name::<FieldType>("field")`.
#[proc_macro_derive(FromRow, attributes(sentinel))]
pub fn derive_fromrow(input: TokenStream) -> TokenStream {
    fromrow::derive_fromrow_impl(input.into()).into()
}

/// `sntl::query!("SQL", params…)` — compile-time-validated query that returns
/// an anonymous record (a private struct with one field per output column).
#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn query(input: TokenStream) -> TokenStream {
    query::anonymous::expand(input.into()).into()
}

/// `sntl::query_as!(Target, "SQL", params…)` — like `query!` but loads each
/// row into the user-supplied `Target: FromRow`.
#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn query_as(input: TokenStream) -> TokenStream {
    query::typed::expand_as(input.into()).into()
}

/// `sntl::query_scalar!("SQL", params…)` — single-column projection.
#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn query_scalar(input: TokenStream) -> TokenStream {
    query::typed::expand_scalar(input.into()).into()
}

/// `sntl::query_file!("queries/foo.sql", params…)` — load SQL from disk.
#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn query_file(input: TokenStream) -> TokenStream {
    query::file::expand(input.into()).into()
}

/// `sntl::query_file_as!(Target, "queries/foo.sql", params…)`.
#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn query_file_as(input: TokenStream) -> TokenStream {
    query::file::expand_as(input.into()).into()
}

/// `sntl::query_unchecked!("SQL", params…)` — escape hatch that skips the
/// `.sentinel/` cache and runs through the driver's untyped `query` path.
#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn query_unchecked(input: TokenStream) -> TokenStream {
    query::unchecked::expand(input.into()).into()
}

/// `sntl::query_as_unchecked!(Target, "SQL", params…)`.
#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn query_as_unchecked(input: TokenStream) -> TokenStream {
    query::unchecked::expand_as(input.into()).into()
}

/// `sntl::query_pipeline!(conn, name1: "SQL"; name2: "SQL" using Target, p1, p2; …)`.
///
/// Sends every entry in a single PipelineBatch round-trip.
#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn query_pipeline(input: TokenStream) -> TokenStream {
    query::pipeline::expand(input.into()).into()
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
