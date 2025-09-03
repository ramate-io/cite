// Integration tests for compile-time behavior of the cite macro

#[test]
fn test_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/*.rs");
}

#[test]
fn test_helper_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/doc_1_pass.rs");
}
