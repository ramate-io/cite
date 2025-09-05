pub mod ui;

use crate::{Comparison, Content, Current, Diff, Id, Referenced, Source, SourceError};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockSource {
	pub id: Id,
	pub referenced_content: String,
	pub current_content: String,
}

impl MockSource {
	/// Create a new MockSource
	pub fn new(referenced: String, current: String) -> Self {
		Self {
			id: Id::new(format!("mock_source_{}", referenced)),
			referenced_content: referenced,
			current_content: current,
		}
	}

	/// Helper for when referenced and current are the same (no diff)
	pub fn same(content: String) -> Self {
		Self::new(content.clone(), content)
	}

	/// Helper for when content has changed
	pub fn changed(referenced: String, current: String) -> Self {
		Self::new(referenced, current)
	}
}

/// Helper function for creating a mock source with same content
/// Convenient for runtime usage with string literals
pub fn mock_source_same(content: &str) -> MockSource {
	MockSource::same(content.to_string())
}

/// Helper function for creating a mock source with changed content  
/// Convenient for runtime usage with string literals
pub fn mock_source_changed(referenced: &str, current: &str) -> MockSource {
	MockSource::changed(referenced.to_string(), current.to_string())
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
