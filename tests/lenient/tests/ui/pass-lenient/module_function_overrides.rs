// Test that module and function overrides are respected when lenient feature is enabled

use cite::cite;

// Should pass with ERROR level override on module
#[cite(mock, changed = ("old content", "new content"), level = "WARN")]
mod test_module {
	pub fn module_function() {
		println!("This should pass with WARN level override on module");
	}
}

// Should pass with ANY annotation override on function
#[cite(mock, same = "content", annotation = "ANY")]
fn function_with_any_annotation() {
	println!("This should pass with ANY annotation override");
}

// Should pass with SILENT level override on function
#[cite(mock, changed = ("old content", "new content"), level = "SILENT")]
fn function_with_silent_override() {
	println!("This should pass with SILENT level override");
}

// Should pass with combined overrides on function
#[cite(mock, changed = ("old content", "new content"), level = "WARN", annotation = "ANY")]
fn function_with_combined_overrides() {
	println!("This should pass with combined overrides");
}

fn main() {
	test_module::module_function();
	function_with_any_annotation();
	function_with_silent_override();
	function_with_combined_overrides();
}
