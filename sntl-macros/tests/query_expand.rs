#[test]
fn query_expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/query/basic.rs");
    t.pass("tests/expand/query/array_basic.rs");
    t.pass("tests/expand/query/array_non_null.rs");
    t.pass("tests/expand/query/tuple_basic.rs");
    t.compile_fail("tests/expand/query/cache_miss.rs");
    t.compile_fail("tests/expand/query/non_null_elements_bad.rs");
    t.compile_fail("tests/expand/query/tuple_arity_bad.rs");
}
