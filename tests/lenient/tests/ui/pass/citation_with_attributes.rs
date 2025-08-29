// Test that citation attributes compile successfully

use cite::cite;

#[cite(
	mock,
	same = "content",
	reason = "This is why we need this reference",
	reason = "test reason"
)]
fn function_with_reason() {}

#[cite(mock, same = "content", level = "WARN", reason = "test reason")]
fn function_with_level() {}

#[cite(mock, same = "content", annotation = "ANY", reason = "test reason")]
fn function_with_annotation() {}

#[cite(
	mock,
	same = "content",
	reason = "Multi",
	level = "ERROR",
	annotation = "FOOTNOTE",
	reason = "test reason"
)]
fn function_with_all_attributes() {}

// Multiple citations should work
#[cite(mock, same = "first", reason = "test reason")]
#[cite(mock, same = "second", reason = "test reason")]
fn function_with_multiple_citations() {}

fn main() {
	function_with_reason();
	function_with_level();
	function_with_annotation();
	function_with_all_attributes();
	function_with_multiple_citations();
}
