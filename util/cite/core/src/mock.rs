use crate::{Content, Referenced, Current, Diff, Source, Comparison, SourceError, Id};

// ==============================================================================
// Concrete Implementations for Testing/Mock Usage
// ==============================================================================

/// Simple string-based content implementation
#[derive(Debug, Clone, PartialEq)]
pub struct StringContent(pub String);

impl Content for StringContent {}

/// Referenced string content - what was originally cited
#[derive(Debug, Clone, PartialEq)]
pub struct ReferencedString(pub String);

impl Content for ReferencedString {}
impl Referenced for ReferencedString {}

/// Current string content - what's currently available
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentString(pub String);

impl Content for CurrentString {}

/// Simple string diff - just tracks if strings are different
#[derive(Debug, Clone, PartialEq)]
pub struct StringDiff {
    pub has_changes: bool,
    pub referenced: String,
    pub current: String,
}

impl Diff for StringDiff {
    fn is_empty(&self) -> bool {
        !self.has_changes
    }
}

impl Current<ReferencedString, StringDiff> for CurrentString {
    fn diff(&self, other: &ReferencedString) -> Result<StringDiff, SourceError> {
        Ok(StringDiff {
            has_changes: self.0 != other.0,
            referenced: other.0.clone(),
            current: self.0.clone(),
        })
    }
}

/// Mock source for testing - compares a static "referenced" string with a "current" string
#[derive(Debug, Clone)]
pub struct MockSource {
    pub id: Id,
    pub referenced_content: String,
    pub current_content: String,
}

impl MockSource {
    /// Create a new MockSource and return a static reference for use in macros
    pub fn new(referenced: &'static str, current: &'static str) -> Self {
        Self {
            id: Id::new(format!("mock_source_{}", referenced)),
            referenced_content: referenced.to_string(),
            current_content: current.to_string(),
        }
    }
    
    /// Helper for when referenced and current are the same (no diff)
    /// Returns &'static Self for use in macros
    pub fn same(content: &'static str) -> Self {
        Self::new(content, content)
    }
    
    /// Helper for when content has changed
    /// Returns &'static Self for use in macros
    pub fn changed(referenced: &'static str, current: &'static str) -> Self {
        Self::new(referenced, current)
    }
}

impl Source<ReferencedString, CurrentString, StringDiff> for MockSource {
    fn id(&self) -> &Id {
        &self.id
    }

    fn get_referenced(&self) -> Result<ReferencedString, SourceError> {
        Ok(ReferencedString(self.referenced_content.clone()))
    }

    fn get_current(&self) -> Result<CurrentString, SourceError> {
        Ok(CurrentString(self.current_content.clone()))
    }

    fn get(&self) -> Result<Comparison<ReferencedString, CurrentString, StringDiff>, SourceError> {
        let referenced = ReferencedString(self.referenced_content.clone());
        let current = CurrentString(self.current_content.clone());
        let diff = current.diff(&referenced)?;
        
        Ok(Comparison::new(referenced, current, diff))
    }
}

// ==============================================================================
// Macro Pattern Matching Support
// ==============================================================================

/// Helper functions for creating MockSource instances from parsed arguments
/// 
/// These functions are used by the procedural macro to construct MockSource
/// instances from parsed string literals.

/// Create a MockSource with the same referenced and current content
pub fn mock_source_same(content: String) -> MockSource {
    MockSource {
        id: Id::new(format!("mock_source_{}", content)),
        referenced_content: content.clone(),
        current_content: content,
    }
}

/// Create a MockSource with different referenced and current content
pub fn mock_source_changed(referenced: String, current: String) -> MockSource {
    MockSource {
        id: Id::new(format!("mock_source_{}", referenced)),
        referenced_content: referenced,
        current_content: current,
    }
}
