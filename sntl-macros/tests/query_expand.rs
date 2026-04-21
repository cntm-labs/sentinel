#[test]
fn query_expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/query/basic.rs");
    t.compile_fail("tests/expand/query/cache_miss.rs");
}
