#[test]
fn fromrow_expands() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/fromrow/basic.rs");
}
