// Integration tests for compile-time behavior of the cite macro

#[test]
fn test_compile_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
}

#[test]
fn test_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}

// Individual test cases for more granular control
#[test]
fn test_basic_citation_compiles() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/basic_citation.rs");
}

#[test]
fn test_citation_attributes_compile() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/citation_with_attributes.rs");
}

#[test]
fn test_missing_source_fails() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/missing_source.rs");
}

#[test]
fn test_invalid_attribute_fails() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/invalid_attribute.rs");
}

#[test]
fn test_wrong_target_fails() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/wrong_target.rs");
}

#[test]
fn test_mock_diff_display_compiles() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/mock_diff_display.rs");
}

#[test]
fn test_behavior_demonstration_compiles() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/behavior_demonstration.rs");
}
