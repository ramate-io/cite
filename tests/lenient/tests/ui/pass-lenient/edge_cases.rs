// Test edge cases that should pass when lenient feature is enabled

use cite::cite;

// Should pass with mixed case annotation override
#[cite(mock, same = "content", annotation = "any")]
fn function_with_lowercase_any() {
	println!("This should pass with lowercase any annotation override");
}

// Should pass with content mismatch but SILENT level override
#[cite(mock, changed = ("old content", "new content"), level = "SILENT")]
fn function_with_silent_content_mismatch() {
	println!("This should pass with content mismatch but SILENT level override");
}

// Should pass with no reason but ANY annotation override
#[cite(mock, same = "content", annotation = "ANY")]
fn function_with_any_no_reason() {
	println!("This should pass with no reason but ANY annotation override");
}

fn main() {
	function_with_lowercase_any();
	function_with_silent_content_mismatch();
	function_with_any_no_reason();
}
