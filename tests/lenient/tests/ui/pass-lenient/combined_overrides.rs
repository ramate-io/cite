// Test that combined level and annotation overrides are respected when lenient feature is enabled

use cite::cite;

// Should pass with SILENT level and FOOTNOTE annotation overrides
#[cite(mock, changed = ("old content", "new content"), level = "SILENT", annotation = "FOOTNOTE", reason = "Testing combined overrides")]
fn function_with_silent_and_footnote() {
	println!("This should pass with SILENT level and FOOTNOTE annotation overrides");
}

// Should pass with WARN level and ANY annotation overrides
#[cite(mock, changed = ("old content", "new content"), level = "WARN", annotation = "ANY")]
fn function_with_warn_and_any() {
	println!("This should pass with WARN level and ANY annotation overrides");
}

fn main() {
	function_with_silent_and_footnote();
	function_with_warn_and_any();
}
