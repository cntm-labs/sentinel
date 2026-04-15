#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/no_primary_key.rs");
    t.compile_fail("tests/compile_fail/duplicate_primary_key.rs");
    t.compile_fail("tests/compile_fail/unknown_attribute.rs");
    t.compile_fail("tests/compile_fail/unloaded_relation_access.rs");
    t.compile_fail("tests/compile_fail/include_required.rs");
}
