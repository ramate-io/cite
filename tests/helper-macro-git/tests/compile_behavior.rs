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
fn test_doc_1_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/doc_1_pass.rs");
}

#[test]
fn test_doc_2_content_diff() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail/doc_2_content_diff.rs");
}
