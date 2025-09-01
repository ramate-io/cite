// Test that struct and trait overrides are respected when lenient feature is enabled

use cite::cite;

// should pass with WARN level override on struct
#[cite(mock, same = "content", level = "WARN")]
struct TestStruct {
	field: String,
}

// Should pass with ANY annotation override on struct
#[cite(mock, same = "content", annotation = "ANY")]
struct AnotherStruct {
	value: i32,
}

// Should pass with SILENT level override on trait
#[cite(mock, changed = ("old content", "new content"), level = "SILENT")]
trait TestTrait {
	fn test_method(&self);
}

// Should pass with combined overrides on impl
#[cite(mock, changed = ("old content", "new content"), level = "WARN", annotation = "ANY")]
impl TestStruct {
	fn new(field: String) -> Self {
		Self { field }
	}
}

fn main() {
	let _struct = TestStruct::new("test".to_string());
}
