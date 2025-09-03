use cite::cite;
use cite_helper_macro_git_test::helper_macro_git;

const DOC_2: u32 = 2;

#[cite(helper_macro_git!(doc = DOC_2), reason = "Testing git source with constant")]
pub fn test_git_source() {
	println!("This function has a citation with a git source");
}

fn main() {
	test_git_source();
}
