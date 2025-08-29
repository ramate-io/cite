// Test that basic citation syntax compiles successfully

use cite::cite;

#[cite(mock, same = "test content", reason = "test reason")]
fn test_function() {
	println!("Hello, world!");
}

#[cite(mock, same = "struct content", reason = "test reason")]
struct TestStruct {
	field: i32,
}

#[cite(mock, same = "trait content", reason = "test reason")]
trait TestTrait {
	fn test_method(&self);
}

#[cite(mock, same = "impl content", reason = "test reason")]
impl TestStruct {
	fn new(field: i32) -> Self {
		Self { field }
	}

	#[cite(mock, same = "method content", reason = "test reason")]
	fn get_field(&self) -> i32 {
		self.field
	}

	#[cite(mock, changed = ("old method", "new method"), level = "SILENT", reason = "test reason")]
	fn set_field(&mut self, value: i32) {
		self.field = value;
	}
}

fn main() {
	test_function();
	let mut test_struct = TestStruct::new(42);
	let _field_value = test_struct.get_field();
	test_struct.set_field(100);
}
