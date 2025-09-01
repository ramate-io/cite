// Test that citations fail when annotation-footnote feature is enabled but no reason provided

use cite::cite;

#[cite(mock, same = "content")]
fn function_without_reason() {
	println!("This should pass when annotationless feature is enabled");
}

fn main() {
	function_without_reason();
}
