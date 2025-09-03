use cite::cite;
use helper_macro_git::helper_macro_git;

#[helper_macro_git(doc = 1)]
#[cite(above, reason = "Testing git source")]
#[helper_macro_git(doc = 2)]
#[cite(
	above,
	reason = "Testing git source 2",
	ref_rev = "41b038e2dcd66b710ccfa2fd3be56426a9625a67"
)]
pub fn test_git_source() {
	println!(
		"This function should fail to compile because doc 2 has changed since the default revision"
	);
}

fn main() {
	test_git_source();
}
