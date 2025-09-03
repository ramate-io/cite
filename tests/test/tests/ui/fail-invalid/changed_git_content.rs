// Test that HTTP citation syntax compiles successfully

use cite::cite;

#[cite(
	git,
	remote = "https://github.com/ramate-io/cite",
	ref_rev = "74aa653664cd90adcc5f836f1777f265c109045b",
	cur_rev = "main",
	path = "README.md",
	reason = "Testing git source"
)]
fn test_function_with_git_citation() {
	println!("This function has an Git citation");
}

fn main() {
	test_function_with_git_citation();
}
