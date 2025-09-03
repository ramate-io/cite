use cite::cite;

/// Test the git source with citation footnote
#[cite(
	git,
	remote = "https://github.com/ramate-io/cite",
	ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
	cur_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
	path = "README.md",
	reason = "Testing git source"
)]
pub fn test_git_source() {
	println!("This function has a citation with a git source");
}

#[cfg(test)]
pub mod tests {
	use cite::cite;
	use cite_core::{mock_source_changed, mock_source_same, Source};

	// Test basic citation on a function
	#[cite(
		mock,
		same = "test content",
		reason = "Testing basic citation functionality",
		reason = "test reason"
	)]
	fn test_function() {
		println!("This function has a citation");
	}

	// Test citation with reason
	#[cite(
		mock,
		same = "important content",
		reason = "This demonstrates why we need this reference"
	)]
	fn test_function_with_reason() {
		println!("This function has a citation with a reason");
	}

	// Test citation with multiple attributes
	#[cite(mock, same = "complex content", reason = "Complex reasoning", level = "WARN")]
	fn test_function_with_multiple_attributes() {
		println!("This function has multiple citation attributes");
	}

	// Test citation on a struct
	#[cite(mock, same = "struct content", reason = "Testing struct citation")]
	struct TestStruct {
		field: String,
	}

	// Test citation on a trait
	#[cite(mock, same = "trait content", reason = "Testing trait citation")]
	pub trait TestTrait {
		fn do_something(&self);
	}

	// Test citation on impl block
	#[cite(mock, same = "impl content", reason = "Testing impl citation")]
	impl TestStruct {
		fn new(field: String) -> Self {
			Self { field }
		}
	}

	// Test the http source
	#[cite(
		http,
		url = "https://jsonplaceholder.typicode.com/todos/1",
		match_type = "full",
		reason = "Testing http source"
	)]
	fn test_http_source() {
		println!("This function has a citation with an http source");
	}

	/// Test the git source with citation footnote
	#[cite(
		git,
		remote = "https://github.com/ramate-io/cite",
		ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
		cur_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
		path = "README.md",
		reason = "Testing git source with new formatting"
	)]
	pub fn test_git_source() {
		println!("This function has a citation with a git source");
	}

	/// Test citation with warning level
	#[cite(
		mock,
		same = "test content",
		level = "WARN",
		reason = "This citation will emit warnings on content mismatch"
	)]
	pub fn test_warning_citation() {
		println!("This function has a citation that will warn");
	}

	/// Test HTTP source with citation footnote
	#[cite(http, url = "https://example.com", pattern = "title", reason = "Testing HTTP source")]
	fn test_http_source_with_footnote() {
		println!("This function has a citation with an HTTP source");
	}

	/// Test mock source with citation footnote
	#[cite(mock, same = "test content", reason = "Testing mock source")]
	fn test_mock_source_with_footnote() {
		println!("This function has a citation with a mock source");
	}

	#[test]
	fn test_basic_functionality() {
		// Test that cited functions can be called normally
		test_function();
		test_function_with_reason();
		test_function_with_multiple_attributes();

		// Test that cited http source works normally
		test_http_source();
		test_http_source_with_footnote();
		test_mock_source_with_footnote();

		// Test that cited git source works normally
		test_git_source();

		// Test that cited structs work normally
		let test_struct = TestStruct::new("test".to_string());
		assert_eq!(test_struct.field, "test");
	}

	#[test]
	fn test_citation_with_changed_content() {
		// This should trigger a compile-time warning/error in debug mode
		#[cite(mock, same = "test content", reason = "Test function with citation")]
		fn function_with_changed_citation() {
			println!("This function references content that has changed");
		}

		// This should still compile and run
		function_with_changed_citation();
	}

	// Test multiple citations on the same item
	#[cite(mock, same = "first reference", reason = "Testing multiple citations")]
	#[cite(mock, same = "second reference", reason = "Testing multiple citations")]
	fn function_with_multiple_citations() {
		println!("This function has multiple citations");
	}

	#[test]
	fn test_multiple_citations() {
		function_with_multiple_citations();
	}

	#[test]
	fn test_mock_source_directly() {
		// This test uses MockSource at runtime, which will satisfy the import analyzer
		let source = mock_source_same("test content");
		let comparison = source.get().expect("Should get comparison");
		assert!(comparison.is_same());

		let changed_source = mock_source_changed("old", "new");
		let changed_comparison = changed_source.get().expect("Should get comparison");
		assert!(!changed_comparison.is_same());
	}

	#[test]
	fn test_behavior_integration() {
		use cite_core::{CitationAnnotation, CitationBehavior, CitationGlobal, CitationLevel};

		// Test with lenient global behavior
		let behavior = CitationBehavior::new(
			CitationLevel::Warn,
			CitationAnnotation::Any,
			CitationGlobal::Lenient,
		);

		// Valid citation should pass
		let valid_source = mock_source_same("content");
		let valid_comparison = valid_source.get().expect("Should get comparison");
		let valid_result = valid_comparison.validate(&behavior, None);
		assert!(valid_result.is_valid());
		assert!(!valid_result.should_fail_compilation());
		assert!(!valid_result.should_report());

		// Invalid citation with default level (warn) should report but not fail
		let invalid_source = mock_source_changed("old", "new");
		let invalid_comparison = invalid_source.get().expect("Should get comparison");
		let invalid_result = invalid_comparison.validate(&behavior, None);
		assert!(!invalid_result.is_valid());
		assert!(!invalid_result.should_fail_compilation());
		assert!(invalid_result.should_report());
		assert_eq!(invalid_result.level(), Some(CitationLevel::Warn));

		// Invalid citation with local error level should fail compilation
		let error_result = invalid_comparison.validate(&behavior, Some(CitationLevel::Error));
		assert!(!error_result.is_valid());
		assert!(error_result.should_fail_compilation());
		assert!(error_result.should_report());
		assert_eq!(error_result.level(), Some(CitationLevel::Error));

		// Invalid citation with local silent level should not report
		let silent_result = invalid_comparison.validate(&behavior, Some(CitationLevel::Silent));
		assert!(!silent_result.is_valid());
		assert!(!silent_result.should_fail_compilation());
		assert!(!silent_result.should_report());
		assert_eq!(silent_result.level(), Some(CitationLevel::Silent));
	}

	#[test]
	fn test_strict_global_behavior() {
		use cite_core::{CitationAnnotation, CitationBehavior, CitationGlobal, CitationLevel};

		// Test with strict global behavior
		let behavior = CitationBehavior::new(
			CitationLevel::Error,
			CitationAnnotation::Any,
			CitationGlobal::Strict,
		);

		let invalid_source = mock_source_changed("old", "new");
		let invalid_comparison = invalid_source.get().expect("Should get comparison");

		// In strict mode, local overrides should be ignored
		let result_with_local_override =
			invalid_comparison.validate(&behavior, Some(CitationLevel::Silent));
		assert!(!result_with_local_override.is_valid());
		assert!(result_with_local_override.should_fail_compilation()); // Should use global Error level
		assert!(result_with_local_override.should_report());
		assert_eq!(result_with_local_override.level(), Some(CitationLevel::Error));
	}
}
