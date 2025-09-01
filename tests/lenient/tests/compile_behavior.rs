// Integration tests for compile-time behavior of the cite macro

#[test]
fn test_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/*.rs");
	// check pass with warning checks as there is no file overloading and some of these should warn
	t.pass_with(
		"tests/ui/pass-lenient/*.rs",
		Vec::<(String, String)>::new(),
		Vec::<String>::new(),
		true,
	);
}

#[test]
fn test_compile_pass_lenient() {
	let t = trybuild::TestCases::new();
	t.pass_with(
		"tests/ui/pass-lenient/*.rs",
		Vec::<(String, String)>::new(),
		Vec::<String>::new(),
		true,
	);
}

#[test]
fn test_compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-syntax/*.rs");
}

#[test]
fn test_compile_fail_syntax() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail-syntax/*.rs");
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
fn test_missing_reason_warn_compiles() {
	let t = trybuild::TestCases::new();
	t.warn("tests/ui/pass-lenient/missing_reason_warn.rs");
}

// New comprehensive lenient override tests
#[test]
fn test_level_overrides_lenient() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass-lenient/level_overrides.rs");
}

#[test]
fn test_annotation_overrides_lenient() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass-lenient/annotation_overrides.rs");
}

#[test]
fn test_combined_overrides_lenient() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass-lenient/combined_overrides.rs");
}

#[test]
fn test_edge_cases_lenient() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass-lenient/edge_cases.rs");
}

#[test]
fn test_struct_trait_overrides_lenient() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass-lenient/struct_trait_overrides.rs");
}

#[test]
fn test_module_function_overrides_lenient() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass-lenient/module_function_overrides.rs");
}
