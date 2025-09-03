use cite::cite;
use helper_macro_git::helper_macro_git;

#[helper_macro_git(doc = 1)]
#[cite(above, reason = "Testing git source")]
pub fn test_git_source() {
	println!("This function has a citation with a git source");
}
