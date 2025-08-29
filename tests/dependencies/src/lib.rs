//! Test to verify that the cite macro has no runtime dependencies
//!
//! This crate uses the cite macro and verifies that no heavy dependencies
//! are pulled in at runtime - all cite functionality should be compile-time only.

use cite::cite;

/// Simple function with a citation
#[cite(mock, same = "test content", reason = "test reason")]
pub fn test_function() {
	println!("Hello world");
}

/// Another function with a citation
#[cite(mock, same = "another function", level = "SILENT", reason = "test reason")]
pub fn another_function() {
	println!("Another function");
}

/// A function that cites an httpbin source
#[cite(
	http,
	url = "https://jsonplaceholder.typicode.com/todos/1",
	match_type = "full",
	reason = "test reason"
)]
pub fn jsonplaceholder_function() {
	println!("jsonplaceholder function");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_functions_work() {
		test_function();
		another_function();
		jsonplaceholder_function();
	}
}
