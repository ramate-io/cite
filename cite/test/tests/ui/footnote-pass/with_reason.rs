// Test that citations pass when annotation-footnote feature is enabled and a reason is provided

use cite::cite;

#[cite(mock, same = "content", reason = "This function implements the documented API")]
fn function_with_reason() {
	println!("This should pass when annotation-footnote feature is enabled");
}

fn main() {
	function_with_reason();
}
