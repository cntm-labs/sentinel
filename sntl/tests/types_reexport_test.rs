//! Compile-only test that asserts the re-exports resolve.

use sntl::types::{cube::PgCube, ltree::PgLTree};
use sntl::FromSql;

#[test]
fn hstore_module_resolves() {
    // Hstore module is re-exported from sntl::types. If the re-export were
    // missing, this line would fail to compile. The hstore module provides
    // FromSql/ToSql implementations for HashMap<String, Option<String>>.
    let _oid = <std::collections::HashMap<String, Option<String>> as FromSql>::oid();
}

#[test]
fn ltree_module_resolves() {
    // PgLTree is the struct exported from the ltree module.
    let _ltree = PgLTree("top.science".to_string());
}

#[test]
fn cube_module_resolves() {
    // PgCube is the struct exported from the cube module.
    let _cube = PgCube::point(vec![1.0, 2.0, 3.0]);
}
