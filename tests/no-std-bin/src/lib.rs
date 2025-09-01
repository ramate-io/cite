#![no_std]

use cite::cite;

#[cite(mock, same = "no-std binary lib function", reason = "test reason")]
pub fn binary_lib_function() {
	// This function should compile in a no-std environment
}

#[cite(mock, same = "no-std binary struct", reason = "test reason")]
pub struct BinaryStruct {
	pub value: u32,
}

#[cite(mock, same = "no-std binary enum", reason = "test reason")]
pub enum BinaryEnum {
	Option1,
	Option2(u32),
}

impl BinaryStruct {
	#[cite(mock, same = "constructor", level = "SILENT", reason = "test reason")]
	pub fn new(value: u32) -> Self {
		Self { value }
	}

	#[cite(mock, same = "getter method", reason = "test reason")]
	pub fn get_value(&self) -> u32 {
		self.value
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_no_std_binary_lib_functions_work() {
		binary_lib_function();
		let s = BinaryStruct::new(42);
		let _value = s.get_value();
		let _enum_val = BinaryEnum::Option1;
	}
}
