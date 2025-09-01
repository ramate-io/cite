// Runtime tests to verify that our behavior system works correctly
// This complements the trybuild tests by testing the behavior logic itself

#[test]
fn test_mock_source_diffs() {
    use cite_core::{mock_source_changed, Source, CitationBehavior, CitationLevel, CitationAnnotation, CitationGlobal};
    
    // Test that MockSource::changed actually creates diffs
    let changed_source = mock_source_changed("original content", "modified content");
    let comparison = changed_source.get().expect("Should get comparison");
    
    assert!(!comparison.is_same(), "Changed source should show differences");
    assert!(comparison.diff().has_changes, "Diff should indicate changes");
    assert_eq!(comparison.diff().referenced, "original content");
    assert_eq!(comparison.diff().current, "modified content");
    
    // Test behavior-driven validation
    let behavior = CitationBehavior::new(
        CitationLevel::Error,
        CitationAnnotation::Any,
        CitationGlobal::Lenient,
    );
    
    let result = comparison.validate(&behavior, None);
    assert!(!result.is_valid());
    assert!(result.should_fail_compilation()); // Error level should fail compilation
    assert!(result.should_report());
    assert_eq!(result.level(), Some(CitationLevel::Error));
}

#[test]
fn test_behavior_level_overrides() {
    use cite_core::{mock_source_changed, Source, CitationBehavior, CitationLevel, CitationAnnotation, CitationGlobal};
    
    let changed_source = mock_source_changed("old", "new");
    let comparison = changed_source.get().expect("Should get comparison");
    
    // Test lenient mode with local overrides
    let lenient_behavior = CitationBehavior::new(
        CitationLevel::Warn,  // Global default
        CitationAnnotation::Any,
        CitationGlobal::Lenient,
    );
    
    // Local override to ERROR should be respected in lenient mode
    let error_result = comparison.validate(&lenient_behavior, Some(CitationLevel::Error));
    assert!(error_result.should_fail_compilation());
    assert_eq!(error_result.level(), Some(CitationLevel::Error));
    
    // Local override to SILENT should be respected in lenient mode  
    let silent_result = comparison.validate(&lenient_behavior, Some(CitationLevel::Silent));
    assert!(!silent_result.should_fail_compilation());
    assert!(!silent_result.should_report());
    assert_eq!(silent_result.level(), Some(CitationLevel::Silent));
    
    // Test strict mode ignores local overrides
    let strict_behavior = CitationBehavior::new(
        CitationLevel::Warn,  // Global default
        CitationAnnotation::Any,
        CitationGlobal::Strict,
    );
    
    // Local override to ERROR should be IGNORED in strict mode
    let strict_result = comparison.validate(&strict_behavior, Some(CitationLevel::Error));
    assert!(!strict_result.should_fail_compilation()); // Uses global WARN, not local ERROR
    assert!(strict_result.should_report());
    assert_eq!(strict_result.level(), Some(CitationLevel::Warn)); // Global level used
}

#[test]
fn test_environment_variable_parsing() {
    use cite_core::{CitationLevel, CitationAnnotation, CitationGlobal};
    
    // Test level parsing
    assert_eq!(CitationLevel::from_str("error").unwrap(), CitationLevel::Error);
    assert_eq!(CitationLevel::from_str("WARN").unwrap(), CitationLevel::Warn);
    assert_eq!(CitationLevel::from_str("silent").unwrap(), CitationLevel::Silent);
    assert!(CitationLevel::from_str("invalid").is_err());
    
    // Test annotation parsing
    assert_eq!(CitationAnnotation::from_str("footnote").unwrap(), CitationAnnotation::Footnote);
    assert_eq!(CitationAnnotation::from_str("ANY").unwrap(), CitationAnnotation::Any);
    assert!(CitationAnnotation::from_str("invalid").is_err());
    
    // Test global parsing
    assert_eq!(CitationGlobal::from_str("strict").unwrap(), CitationGlobal::Strict);
    assert_eq!(CitationGlobal::from_str("LENIENT").unwrap(), CitationGlobal::Lenient);
    assert!(CitationGlobal::from_str("invalid").is_err());
}

#[test]
fn test_diff_content_display() {
    use cite_core::{mock_source_changed, Source};
    
    // Test that we can access diff details for display
    let source = mock_source_changed(
        "function old_api() -> Result<(), Error>", 
        "function new_api() -> Result<String, MyError>"
    );
    let comparison = source.get().expect("Should get comparison");
    
    assert!(!comparison.is_same());
    
    let diff = comparison.diff();
    assert!(diff.has_changes);
    assert!(diff.referenced.contains("old_api"));
    assert!(diff.current.contains("new_api"));
    assert!(diff.referenced.contains("Error"));
    assert!(diff.current.contains("MyError"));
    
    // In a real implementation, we could format this nicely:
    println!("Diff detected:");
    println!("  Referenced: {}", diff.referenced);
    println!("  Current:    {}", diff.current);
    
    // This shows that our mock system is ready to display meaningful diffs
}
