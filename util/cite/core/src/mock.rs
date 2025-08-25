use crate::{Content, Referenced, Current, Diff, Source, Comparison, SourceError};

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
    pub referenced_content: String,
    pub current_content: String,
}

impl MockSource {
    /// Create a new MockSource and return a static reference for use in macros
    pub fn new(referenced: &'static str, current: &'static str) -> &'static Self {
        Box::leak(Box::new(Self {
            referenced_content: referenced.to_string(),
            current_content: current.to_string(),
        }))
    }
    
    /// Helper for when referenced and current are the same (no diff)
    /// Returns &'static Self for use in macros
    pub fn same(content: &'static str) -> &'static Self {
        Self::new(content, content)
    }
    
    /// Helper for when content has changed
    /// Returns &'static Self for use in macros
    pub fn changed(referenced: &'static str, current: &'static str) -> &'static Self {
        Self::new(referenced, current)
    }
}

/// Compile-time friendly mock source using static string references
#[derive(Debug, Clone, Copy)]
pub struct StaticMockSource {
    pub referenced_content: &'static str,
    pub current_content: &'static str,
}

impl StaticMockSource {
    /// Create a new static mock source - can be used in const contexts
    pub const fn new(referenced: &'static str, current: &'static str) -> Self {
        Self {
            referenced_content: referenced,
            current_content: current,
        }
    }
    
    /// Helper for when referenced and current are the same (no diff) - const fn
    pub const fn same(content: &'static str) -> Self {
        Self::new(content, content)
    }
    
    /// Helper for when content has changed - const fn
    pub const fn changed(referenced: &'static str, current: &'static str) -> Self {
        Self::new(referenced, current)
    }
}

/// Constructor functions that return static references for use in macros
/// 
/// These functions can be called in cite macro expressions and return static references
/// to sources. How the static references are created is up to the implementer.

/// Create a static mock source with no differences
/// 
/// Usage: `#[cite(mock_same("content"))]`
pub fn mock_same(content: &'static str) -> &'static StaticMockSource {
    // Leak memory to create a static reference for simplicity
    // Real implementations might use different strategies
    Box::leak(Box::new(StaticMockSource::same(content)))
}

/// Create a static mock source with differences
/// 
/// Usage: `#[cite(mock_changed("old", "new"))]`
pub fn mock_changed(referenced: &'static str, current: &'static str) -> &'static StaticMockSource {
    Box::leak(Box::new(StaticMockSource::changed(referenced, current)))
}

/// Create a static mock source with custom content
/// 
/// Usage: `#[cite(mock_source("referenced", "current"))]`
pub fn mock_source(referenced: &'static str, current: &'static str) -> &'static StaticMockSource {
    Box::leak(Box::new(StaticMockSource::new(referenced, current)))
}

impl Source<ReferencedString, CurrentString, StringDiff> for MockSource {
    fn get(&self) -> Result<Comparison<ReferencedString, CurrentString, StringDiff>, SourceError> {
        let referenced = ReferencedString(self.referenced_content.clone());
        let current = CurrentString(self.current_content.clone());
        let diff = current.diff(&referenced)?;
        
        Ok(Comparison::new(referenced, current, diff))
    }
}

impl Source<ReferencedString, CurrentString, StringDiff> for StaticMockSource {
    fn get(&self) -> Result<Comparison<ReferencedString, CurrentString, StringDiff>, SourceError> {
        let referenced = ReferencedString(self.referenced_content.to_string());
        let current = CurrentString(self.current_content.to_string());
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
        referenced_content: content.clone(),
        current_content: content,
    }
}

/// Create a MockSource with different referenced and current content
pub fn mock_source_changed(referenced: String, current: String) -> MockSource {
    MockSource {
        referenced_content: referenced,
        current_content: current,
    }
}
