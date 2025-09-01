// Test that level overrides are respected when lenient feature is enabled

use cite::cite;

// Should pass with SILENT level override (overrides global warn)
#[cite(mock, changed = ("old content", "new content"), level = "SILENT")]
fn function_with_silent_override() {
	println!("This should pass with SILENT level override");
}

// Should pass with WARN level override (same as global)
#[cite(mock, changed = ("old content", "new content"), level = "WARN")]
fn function_with_warn_override() {
	println!("This should pass with WARN level override");
}

fn main() {
	function_with_silent_override();
	function_with_warn_override();
}
