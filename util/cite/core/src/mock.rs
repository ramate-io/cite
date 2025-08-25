//! Mock source implementation for testing citations
//! 
//! This module provides a simple mock source that works in no_std environments
//! and is useful for testing the citation system.

use crate::{Source, SourceError, Referenced, Current, Diff, Content, Id};

/// Simple mock content that works in no_std
#[derive(Debug, Clone, PartialEq)]
pub struct MockContent {
    data: [u8; 128],
    len: usize,
}

impl Content for MockContent {}
impl Referenced for MockContent {}

impl MockContent {
    pub fn new(data: &str) -> Self {
        let bytes = data.as_bytes();
        let mut buffer = [0u8; 128];
        let len = bytes.len().min(128);
        buffer[..len].copy_from_slice(&bytes[..len]);
        Self { data: buffer, len }
    }
    
    pub fn data(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }
}

/// Mock diff implementation
#[derive(Debug, Clone, PartialEq)]
pub struct MockDiff {
    pub changed: bool,
}

impl Diff for MockDiff {
    fn is_empty(&self) -> bool {
        !self.changed
    }
}

/// Mock current content
#[derive(Debug, Clone, PartialEq)]
pub struct MockCurrentContent {
    data: [u8; 128],
    len: usize,
}

impl Content for MockCurrentContent {}

impl Current<MockContent, MockDiff> for MockCurrentContent {
    fn diff(&self, referenced: &MockContent) -> Result<MockDiff, SourceError> {
        Ok(MockDiff {
            changed: self.data() != referenced.data(),
        })
    }
}

impl MockCurrentContent {
    pub fn new(data: &str) -> Self {
        let bytes = data.as_bytes();
        let mut buffer = [0u8; 128];
        let len = bytes.len().min(128);
        buffer[..len].copy_from_slice(&bytes[..len]);
        Self { data: buffer, len }
    }
    
    pub fn data(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }
}

/// Mock source for testing
pub struct MockSource {
    id: Id,
    referenced: MockContent,
    current: MockCurrentContent,
}

impl MockSource {
    pub fn same(content: &str) -> Self {
        let id = Id::new(content);
        Self {
            id,
            referenced: MockContent::new(content),
            current: MockCurrentContent::new(content),
        }
    }
    
    pub fn changed(referenced: &str, current: &str) -> Self {
        let id = Id::new(referenced);
        Self {
            id,
            referenced: MockContent::new(referenced),
            current: MockCurrentContent::new(current),
        }
    }
}

impl Source<MockContent, MockCurrentContent, MockDiff> for MockSource {
    fn id(&self) -> &Id {
        &self.id
    }
    
    fn get_referenced(&self) -> Result<MockContent, SourceError> {
        Ok(self.referenced.clone())
    }
    
    fn get_current(&self) -> Result<MockCurrentContent, SourceError> {
        Ok(self.current.clone())
    }
}

/// Helper function for creating a mock source with same content
pub fn mock_source_same(content: &str) -> MockSource {
    MockSource::same(content)
}

/// Helper function for creating a mock source with changed content
pub fn mock_source_changed(referenced: &str, current: &str) -> MockSource {
    MockSource::changed(referenced, current)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_source_same() {
        let source = MockSource::same("test content");
        let comparison = source.get().unwrap();
        
        assert_eq!(comparison.referenced().data(), "test content");
        assert_eq!(comparison.current().data(), "test content");
        assert!(!comparison.diff().changed);
        assert!(comparison.is_same());
    }
    
    #[test]
    fn test_mock_source_changed() {
        let source = MockSource::changed("old content", "new content");
        let comparison = source.get().unwrap();
        
        assert_eq!(comparison.referenced().data(), "old content");
        assert_eq!(comparison.current().data(), "new content");
        assert!(comparison.diff().changed);
        assert!(!comparison.is_same());
    }
}
