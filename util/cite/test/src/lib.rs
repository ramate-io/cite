use cite_util::{cite, mock::MockSource};

#[cfg(test)]
mod tests {
    use super::*;

    // Test basic citation on a function
    #[cite(MockSource::same("test content"))]
    fn test_function_with_citation() {
        println!("This function has a citation");
    }

    // Test citation with reason
    #[cite(MockSource::same("important content"), reason = "This demonstrates why we need this reference")]
    fn test_function_with_reason() {
        println!("This function has a citation with a reason");
    }

    // Test citation with multiple attributes
    #[cite(MockSource::same("complex content"), reason = "Complex reasoning", level = "WARN")]
    fn test_function_with_multiple_attributes() {
        println!("This function has multiple citation attributes");
    }

    // Test citation on a struct
    #[cite(MockSource::same("struct content"))]
    struct TestStruct {
        field: String,
    }

    // Test citation on a trait
    #[cite(MockSource::same("trait content"))]
    trait TestTrait {
        fn do_something(&self);
    }

    // Test citation on impl block
    #[cite(MockSource::same("impl content"))]
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
        #[cite(MockSource::changed("original content", "modified content"))]
        fn function_with_changed_citation() {
            println!("This function references content that has changed");
        }

        // This should still compile and run
        function_with_changed_citation();
    }

    // Test multiple citations on the same item
    #[cite(MockSource::same("first reference"))]
    #[cite(MockSource::same("second reference"))]
    fn function_with_multiple_citations() {
        println!("This function has multiple citations");
    }

    #[test]
    fn test_multiple_citations() {
        function_with_multiple_citations();
    }
}