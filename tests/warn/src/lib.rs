use cite::cite;

/// Test the git source with citation footnote
#[cite(
	git,
	remote = "https://github.com/ramate-io/cite",
	ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
	cur_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
	path = "README.md",
	reason = r"
### Testing git source

With some markdown, here's a link [to the git source](https://github.com/ramate-io/cite/blob/94dab273cf6c2abe8742d6d459ad45c96ca9b694/README.md#L1)."
)]
#[cite(mock, same = "test content")]
#[cite(mock, changed = ("a", "b"))]
pub fn test_git_source() {
	println!("This function has a citation with a git source");
}

#[cfg(test)]
pub mod tests {}
