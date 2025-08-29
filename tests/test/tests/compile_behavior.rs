// Integration tests for compile-time behavior of the cite macro

#[test]
fn test_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/*.rs");
}

#[test]
fn test_compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-syntax/*.rs");
	t.compile_fail("tests/ui/fail-invalid/*.rs");
}

#[test]
fn test_compile_fail_syntax() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-syntax/*.rs");
}

#[test]
fn test_compile_fail_invalid() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-invalid/*.rs");
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
	t.compile_fail("tests/ui/fail-syntax/missing_source.rs");
}

#[test]
fn test_invalid_attribute_fails() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-syntax/invalid_attribute.rs");
}

#[test]
fn test_wrong_target_fails() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-syntax/wrong_target.rs");
}

#[test]
fn test_module_citation_compiles() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/module_citation.rs");
}

#[test]
fn test_changed_content_error_fails() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-invalid/changed_content_error.rs");
}

#[test]
fn test_local_override_ignored() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-invalid/local_override_ignored.rs");
}

#[test]
fn test_missing_reason_fails() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-invalid/missing_reason.rs");
}
