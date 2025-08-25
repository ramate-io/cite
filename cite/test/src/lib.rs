#[cfg(test)]
pub mod tests {
    use cite::cite;
    use cite_core::{Source, mock_source_same, mock_source_changed};

    // Test basic citation on a function
    #[cite(mock(same("test content")))]
    fn test_function_with_citation() {
        println!("This function has a citation");
    }

    // Test citation with reason
    #[cite(mock(same("important content")), reason = "This demonstrates why we need this reference")]
    fn test_function_with_reason() {
        println!("This function has a citation with a reason");
    }

    // Test citation with multiple attributes
    #[cite(mock(same("complex content")), reason = "Complex reasoning", level = "WARN")]
    fn test_function_with_multiple_attributes() {
        println!("This function has multiple citation attributes");
    }

    // Test citation on a struct
    #[cite(mock(same("struct content")))]
    struct TestStruct {
        field: String,
    }

    // Test citation on a trait
    #[cite(mock(same("trait content")))]
    pub trait TestTrait {
        fn do_something(&self);
    }

    // Test citation on impl block
    #[cite(mock(same("impl content")))]
    impl TestStruct {
        fn new(field: String) -> Self {
            Self { field }
        }
    }

    #[test]
    fn test_basic_functionality() {
        // Test that cited functions can be called normally
        test_function_with_citation();
        test_function_with_reason();
        test_function_with_multiple_attributes();

        // Test that cited structs work normally
        let test_struct = TestStruct::new("test".to_string());
        assert_eq!(test_struct.field, "test");
    }

    #[test]
    fn test_citation_with_changed_content() {
        // This should trigger a compile-time warning/error in debug mode
        #[cite(mock(changed("original content", "modified content")))]
        fn function_with_changed_citation() {
            println!("This function references content that has changed");
        }

        // This should still compile and run
        function_with_changed_citation();
    }

    // Test multiple citations on the same item
    #[cite(mock(same("first reference")))]
    #[cite(mock(same("second reference")))]
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
        use cite_core::{CitationBehavior, CitationLevel, CitationAnnotation, CitationGlobal};
        
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
        use cite_core::{CitationBehavior, CitationLevel, CitationAnnotation, CitationGlobal};
        
        // Test with strict global behavior
        let behavior = CitationBehavior::new(
            CitationLevel::Error,
            CitationAnnotation::Any,
            CitationGlobal::Strict,
        );
        
        let invalid_source = mock_source_changed("old", "new");
        let invalid_comparison = invalid_source.get().expect("Should get comparison");
        
        // In strict mode, local overrides should be ignored
        let result_with_local_override = invalid_comparison.validate(&behavior, Some(CitationLevel::Silent));
        assert!(!result_with_local_override.is_valid());
        assert!(result_with_local_override.should_fail_compilation()); // Should use global Error level
        assert!(result_with_local_override.should_report());
        assert_eq!(result_with_local_override.level(), Some(CitationLevel::Error));
    }
}