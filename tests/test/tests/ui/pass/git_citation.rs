// Test that HTTP citation syntax compiles successfully

use cite::cite;

#[cite(
	git,
	remote = "https://github.com/ramate-io/cite",
	ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
	cur_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
	path = "README.md",
	reason = "Testing git source"
)]
fn test_function_with_git_citation() {
	println!("This function has an Git citation");
}

fn main() {
	test_function_with_git_citation();
}
