use cite::cite;
use helper_macro_git::helper_macro_git;

#[helper_macro_git(doc = 1)]
#[cite(above, reason = "Testing git source")]
#[helper_macro_git(doc = 2)]
#[cite(above, reason = "Testing git source 2")]
pub fn test_git_source() {
	println!("This function has a citation with a git source");
}

fn main() {
	test_git_source();
}
