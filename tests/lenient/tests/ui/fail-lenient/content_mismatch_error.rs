// Test that content mismatches fail when ERROR level is used, even under lenient flag

use cite::cite;

#[cite(mock, changed = ("old content", "new content"), level = "ERROR")]
fn function_with_content_mismatch() {
	println!("This should fail because ERROR level fails on content mismatches");
}

fn main() {
	function_with_content_mismatch();
}
