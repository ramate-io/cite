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
    pub fn new(referenced: impl Into<String>, current: impl Into<String>) -> Self {
        Self {
            referenced_content: referenced.into(),
            current_content: current.into(),
        }
    }
    
    /// Helper for when referenced and current are the same (no diff)
    pub fn same(content: impl Into<String>) -> Self {
        let content = content.into();
        Self::new(content.clone(), content)
    }
    
    /// Helper for when content has changed
    pub fn changed(referenced: impl Into<String>, current: impl Into<String>) -> Self {
        Self::new(referenced, current)
    }
}

impl Source<ReferencedString, CurrentString, StringDiff> for MockSource {
    fn get(&self) -> Result<Comparison<ReferencedString, CurrentString, StringDiff>, SourceError> {
        let referenced = ReferencedString(self.referenced_content.clone());
        let current = CurrentString(self.current_content.clone());
        let diff = current.diff(&referenced)?;
        
        Ok(Comparison::new(referenced, current, diff))
    }
}
