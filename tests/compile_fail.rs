#[test]
fn rejects_invalid_macro_usage() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
