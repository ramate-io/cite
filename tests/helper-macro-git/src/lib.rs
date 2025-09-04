use cite::cite;
use helper_macro_git::helper_macro_git;

/// Test the git source with citation footnote
#[helper_macro_git(doc = 1)]
#[cite(above, reason = "Testing git source")]
#[helper_macro_git(doc = 2)]
#[cite(above, reason = "Testing git source 2")]
pub fn test_git_source() {
	println!("This function has a citation with a git source");
}
