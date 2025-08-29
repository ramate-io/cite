// Test that citations pass when annotation-footnote feature is enabled and a reason is provided

use cite::cite;

#[cite(mock, same = "content", annotation = "ANY")]
fn function_without_reason() {
	println!("This should pass because lenient feature is enabled and annotation = ANY level is respected");
}

fn main() {
	function_without_reason();
}
