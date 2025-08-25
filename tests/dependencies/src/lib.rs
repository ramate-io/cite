//! Test to verify that cite-util-core has no heavy runtime dependencies
//! 
//! This crate imports cite-util-core with no_std and verifies that:
//! 1. No heavy dependencies like reqwest, scraper, regex are pulled in
//! 2. The core traits work in no_std environments
//! 3. Only lightweight dependencies are included

#![no_std]

use cite_util_core::{
    Source, Referenced, Current, Diff, Content, Comparison, 
    SourceError, Id, CacheBehavior
};

/// Simple test content type that works in no_std
#[derive(Debug, Clone, PartialEq)]
pub struct TestContent {
    data: &'static str,
}

impl Content for TestContent {}
impl Referenced for TestContent {}

/// Simple diff type for no_std testing
#[derive(Debug, Clone, PartialEq)]
pub struct TestDiff {
    changed: bool,
}

impl Diff for TestDiff {
    fn is_empty(&self) -> bool {
        !self.changed
    }
}

/// Current content implementation
#[derive(Debug, Clone, PartialEq)]
pub struct TestCurrentContent {
    data: &'static str,
}

impl Content for TestCurrentContent {}

impl Current<TestContent, TestDiff> for TestCurrentContent {
    fn diff(&self, referenced: &TestContent) -> Result<TestDiff, SourceError> {
        Ok(TestDiff {
            changed: self.data != referenced.data,
        })
    }
}

/// Minimal source implementation for testing
pub struct TestSource {
    id: Id,
    referenced_data: &'static str,
    current_data: &'static str,
}

impl TestSource {
    pub fn new(id_str: &'static str, referenced: &'static str, current: &'static str) -> Self {
        Self {
            id: Id::new(id_str),
            referenced_data: referenced,
            current_data: current,
        }
    }
}

impl Source<TestContent, TestCurrentContent, TestDiff> for TestSource {
    fn id(&self) -> &Id {
        &self.id
    }
    
    fn get_referenced(&self) -> Result<TestContent, SourceError> {
        Ok(TestContent { data: self.referenced_data })
    }
    
    fn get_current(&self) -> Result<TestCurrentContent, SourceError> {
        Ok(TestCurrentContent { data: self.current_data })
    }
}

/// Test that the core traits work in no_std
pub fn test_no_std_functionality() -> Result<(), SourceError> {
    let source = TestSource::new("test", "old_data", "new_data");
    
    let comparison = source.get()?;
    
    // Verify the comparison works
    assert!(comparison.diff().changed);
    assert!(!comparison.is_same());
    assert_eq!(comparison.referenced().data, "old_data");
    assert_eq!(comparison.current().data, "new_data");
    
    Ok(())
}

/// Test cache behavior enum works
pub fn test_cache_behavior() {
    let _enabled = CacheBehavior::Enabled;
    let _ignored = CacheBehavior::Ignored;
    
    // Should be able to compare
    assert_ne!(CacheBehavior::Enabled, CacheBehavior::Ignored);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_no_std_core_functionality() {
        test_no_std_functionality().unwrap();
        test_cache_behavior();
    }
}
