#[test]
fn macro_compile_failures_are_clear() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/agent_tool_missing_version.rs");
}
