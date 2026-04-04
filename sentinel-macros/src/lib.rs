//! Sentinel Macros — derive(Model), derive(Partial), #[reducer].

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
    // Stub — full implementation in Task 2
    let _ = input;
    TokenStream::new()
}
