// Test that annotation overrides are respected when lenient feature is enabled

use cite::cite;

// Should pass with ANY annotation override (overrides global footnote requirement)
#[cite(mock, same = "content", annotation = "ANY")]
fn function_with_any_annotation() {
	println!("This should pass with ANY annotation override");
}

// Should pass with FOOTNOTE annotation override (same as global)
#[cite(mock, same = "content", annotation = "FOOTNOTE", reason = "Testing footnote override")]
fn function_with_footnote_annotation() {
	println!("This should pass with FOOTNOTE annotation override");
}

// Should pass with no annotation when ANY is specified
#[cite(mock, same = "content", annotation = "ANY")]
fn function_with_no_annotation_needed() {
	println!("This should pass without reason when ANY annotation is specified");
}

fn main() {
	function_with_any_annotation();
	function_with_footnote_annotation();
	function_with_no_annotation_needed();
}
