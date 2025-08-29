// Test that enum citations compile successfully

use cite::cite;

/// Test enum with citation
#[cite(mock, same = "test enum with citation", reason = "test reason")]
#[derive(Debug)]
enum TestEnum {
	Variant1,
	Variant2(u32),
	Variant3 { data: String },
}

/// Error enum with citation
#[cite(mock, changed = ("old error", "new error"), level = "SILENT", reason = "test reason")]
#[derive(Debug)]
enum ErrorType {
	NetworkError,
	ParseError,
	IoError,
}

fn main() {
	// Test that we can use all enum variants
	let _variant1 = TestEnum::Variant1;
	let _variant2 = TestEnum::Variant2(42);
	let _variant3 = TestEnum::Variant3 { data: "test".to_string() };

	let _error_val = ErrorType::NetworkError;

	println!("Enum citations compiled successfully!");
}
