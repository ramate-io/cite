// Test that mock sources with diffs compile but can show diff information

use cite::cite;

// This should compile successfully (content is the same)
#[cite(mock, same = "unchanged content", reason = "test reason")]
fn function_with_same_content() {
	println!("No changes here");
}

// This should also compile (our macro doesn't fail compilation by default)
// but demonstrates that diffs are being tracked
#[cite(mock, changed = ("original version", "updated version"), reason = "test reason")]
fn function_with_changed_content() {
	println!("This has a diff in the citation");
}

// Test with reason and level attributes on changed content
#[cite(mock, changed = ("old API", "new API"), reason = "API evolution tracking", level = "WARN")]
fn function_with_detailed_diff() {
	println!("API changed but marked appropriately");
}

fn main() {
	function_with_same_content();
	function_with_changed_content();
	function_with_detailed_diff();
}
