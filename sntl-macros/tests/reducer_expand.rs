#[test]
fn reducer_expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/reducer/basic.rs");
    t.pass("tests/expand/reducer/with_isolation.rs");
    t.compile_fail("tests/compile_fail/reducer_not_async.rs");
    t.compile_fail("tests/compile_fail/reducer_no_conn_arg.rs");
    t.compile_fail("tests/compile_fail/reducer_bad_isolation.rs");
}
