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

	// Test that citations fail when no reason provided (default behavior)
	t.compile_fail("tests/ui/footnote-fail/*.rs");

	// Test that citations pass when reason is provided
	t.pass("tests/ui/footnote-pass/*.rs");
}

#[test]
fn test_strict_fail() {
	let t = trybuild::TestCases::new();

	// Test that local overrides are ignored when using default strict behavior
	t.compile_fail("tests/ui/strict-fail/*.rs");
}

#[test]
fn test_lenient_footnote_pass() {
	let t = trybuild::TestCases::new();

	// Test that citations pass when annotation-footnote feature is enabled and a reason is provided
	t.pass_with(
		"tests/ui/lenient-footnote-pass/*.rs",
		Vec::<(String, String)>::new(),
		vec!["cite/lenient"],
	);
}

#[test]
fn test_annotationless_feature() {
	let t = trybuild::TestCases::new();

	// Test that citations pass without reason when annotationless feature is enabled
	t.pass_with(
		"tests/ui/annotationless-pass/*.rs",
		Vec::<(String, String)>::new(),
		vec!["cite/annotationless"],
	);
}

#[test]
fn test_lenient_feature() {
	let t = trybuild::TestCases::new();

	// Test that local overrides are respected when lenient feature is enabled
	t.pass_with("tests/ui/lenient-pass/*.rs", Vec::<(String, String)>::new(), vec!["cite/lenient"]);
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
