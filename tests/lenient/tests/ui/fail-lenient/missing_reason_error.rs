// Test that citations fail when annotation-footnote feature is enabled but no reason provided

use cite::cite;

#[cite(mock, same = "content", level = "ERROR")]
fn function_without_reason() {
	println!("This should fail because lenient allows it.");
}

fn main() {
	function_without_reason();
}
