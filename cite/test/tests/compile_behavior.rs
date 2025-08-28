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

#[test]
fn test_footnote_annotation_requirements() {
	let t = trybuild::TestCases::new();

	// Test that citations fail when annotation-footnote feature is enabled but no reason provided
	t.compile_fail_with(
		"tests/ui/footnote-fail/*.rs",
		Vec::<(String, String)>::new(),
		vec!["cite/annotation-footnote"],
	);

	// Test that citations pass when annotation-footnote feature is enabled and reason is provided
	t.pass_with(
		"tests/ui/footnote-pass/*.rs",
		Vec::<(String, String)>::new(),
		vec!["cite/annotation-footnote"],
	);
}

#[test]
fn test_global_behavior_strict() {
	let t = trybuild::TestCases::new();

	// Test that local overrides are ignored when global-strict feature is enabled
	t.compile_fail_with(
		"tests/ui/global-strict-fail/*.rs",
		Vec::<(String, String)>::new(),
		vec!["cite/level-warn", "cite/global-strict"],
	);
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

#[test]
fn test_module_citation_compiles() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/module_citation.rs");
}

#[test]
fn test_changed_content_error_fails() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail/changed_content_error.rs");
}

#[test]
fn test_changed_content_silent_compiles() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/changed_content_silent.rs");
}
