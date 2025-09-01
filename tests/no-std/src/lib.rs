//! Test to verify that the cite macro works in no-std environments
//!
//! This crate uses the cite macro in a no-std context and verifies that:
//! 1. The macro compiles successfully in no-std
//! 2. No heavy dependencies are pulled in at runtime
//! 3. The generated code works in embedded/no-std environments

#![no_std]

use cite::cite;

/// Simple function with a citation in no-std context
#[cite(mock, same = "test content", reason = "test reason")]
pub fn no_std_function() {
	// In no-std, we can't use println! so we'll use a simple operation
	let _value = 42;
}

/// Another function with a citation and different parameters
#[cite(mock, same = "another no std function", level = "SILENT", reason = "test reason")]
pub fn another_no_std_function() {
	let _array = [1, 2, 3, 4, 5];
}

/// Function with reason parameter
#[cite(mock, same = "api reference", reason = "depends on external API")]
pub fn api_reference_function() {
	let _result = core::mem::size_of::<usize>();
}

/// Struct with citation in no-std
#[cite(mock, same = "data structure definition", reason = "test reason")]
pub struct NoStdStruct {
	pub field: u32,
}

/// Trait with citation in no-std
#[cite(mock, same = "trait definition", reason = "test reason")]
pub trait NoStdTrait {
	fn no_std_method(&self) -> u32;
}

impl NoStdTrait for NoStdStruct {
	fn no_std_method(&self) -> u32 {
		self.field
	}
}

/// Module with citation
#[cite(mock, same = "module content", reason = "test reason")]
pub mod no_std_module {
	use cite::cite;

	#[cite(mock, same = "inner function", reason = "test reason")]
	pub fn inner_function() {
		let _x = 100;
	}
}

/// A more complex example with const generics that should work in no-std
#[cite(mock, same = "generic const array", reason = "test reason")]
pub fn generic_array_function<const N: usize>() -> [u8; N] {
	[0; N]
}

/// Test that demonstrates cite works with embedded-style code
#[cite(mock, same = "embedded pattern", reason = "test reason")]
pub fn embedded_style_function() -> Option<u32> {
	// Simulate embedded-style error handling without std
	if true {
		Some(42)
	} else {
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_no_std_functions_work() {
		no_std_function();
		another_no_std_function();
		api_reference_function();

		let test_struct = NoStdStruct { field: 123 };
		assert_eq!(test_struct.no_std_method(), 123);

		no_std_module::inner_function();

		let array: [u8; 4] = generic_array_function();
		assert_eq!(array.len(), 4);

		assert_eq!(embedded_style_function(), Some(42));
	}
}
