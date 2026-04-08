//! Tests for Transaction type — compile-time and API checks.
//! Actual execution requires a live database (integration tests).

use sntl::core::Transaction;

#[test]
fn transaction_is_exported_from_core() {
    // Verify Transaction is accessible from the core module.
    fn _assert_type(_: &Transaction<'_>) {}
}

#[test]
fn transaction_is_exported_from_prelude() {
    use sntl::prelude::*;
    // Verify Transaction is accessible from prelude.
    fn _assert_type(_: &Transaction<'_>) {}
}
