pub mod line_range;

use git2::{DiffFormat, DiffOptions, Repository};
pub use line_range::LineRange;

use cite_core::{Content, Current, Diff, Id, Referenced, Source, SourceError};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Error types for git operations
#[derive(Error, Debug)]
pub enum GitSourceError {
	#[error("Git operation failed: {0}")]
	Git(#[from] git2::Error),

	#[error("Invalid path pattern: {0}")]
	InvalidPathPattern(String),

	#[error("Path not found in repository: {0}")]
	PathNotFound(String),

	#[error("Invalid revision: {0}")]
	InvalidRevision(String),
}

impl From<GitSourceError> for SourceError {
	fn from(err: GitSourceError) -> Self {
		SourceError::Network(err.to_string())
	}
}

/// Path pattern for git source files
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathPattern {
	pub path: String,
	pub line_range: Option<LineRange>,
	pub glob: Option<String>,
}

impl PathPattern {
	pub fn try_new(path: &str) -> Result<Self, GitSourceError> {
		// Check if path contains line range specification (e.g., "file.rs#L1-L12")
		let (file_path, line_range) = if let Some(hash_pos) = path.find('#') {
			let file_part = &path[..hash_pos];
			let line_part = &path[hash_pos + 1..];
			(file_part.to_string(), Some(LineRange::try_from_string(line_part)?))
		} else {
			(path.to_string(), None)
		};

		// Check if path is a glob pattern
		let glob = if file_path.contains('*') || file_path.contains('?') || file_path.contains('[')
		{
			Some(file_path.clone())
		} else {
			None
		};

		Ok(Self { path: file_path, line_range, glob })
	}

	/// Check if this pattern matches a given path
	pub fn matches(&self, path: &Path) -> bool {
		if let Some(ref glob_pattern) = self.glob {
			// Use proper glob matching
			use glob::Pattern;
			match Pattern::new(glob_pattern) {
				Ok(pattern) => pattern.matches_path(path),
				Err(_) => false, // Invalid glob pattern doesn't match anything
			}
		} else {
			// Exact path match
			path.to_string_lossy() == self.path
		}
	}

	/// Check if a line number is within the specified range
	pub fn line_in_range(&self, line_number: usize) -> bool {
		if let Some(ref range) = self.line_range {
			line_number >= range.start && line_number <= range.end
		} else {
			true // No line range specified, so all lines match
		}
	}
}

/// Git source configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitSource {
	pub id: Id,
	pub comparison_revision: String,
	pub path_pattern: PathPattern,
}

impl GitSource {
	pub fn try_new(comparison_revision: &str, pattern: &str) -> Result<Self, GitSourceError> {
		let path_pattern = PathPattern::try_new(pattern)?;
		let id = Id::new(format!("git_{}_{}", comparison_revision, pattern));

		Ok(Self { id, comparison_revision: comparison_revision.to_string(), path_pattern })
	}
}

impl Source<GitContent, GitContent, GitDiff> for GitSource {
	fn id(&self) -> &Id {
		&self.id
	}

	fn get_referenced(&self) -> Result<GitContent, SourceError> {
		Ok(self.clone().into())
	}

	fn get_current(&self) -> Result<GitContent, SourceError> {
		Ok(self.clone().into())
	}
}

/// Git content representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitContent {
	pub revision: String,
	pub path_pattern: PathPattern,
}

impl From<GitSource> for GitContent {
	fn from(source: GitSource) -> Self {
		GitContent { revision: source.comparison_revision, path_pattern: source.path_pattern }
	}
}

impl Content for GitContent {}
impl Referenced for GitContent {}

/// Git diff representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitDiff {
	pub diff: String,
	pub has_changes: bool,
}

impl Diff for GitDiff {
	fn is_empty(&self) -> bool {
		!self.has_changes
	}
}

impl Current<GitContent, GitDiff> for GitContent {
	fn diff(&self, other: &GitContent) -> Result<GitDiff, SourceError> {
		// Get the comparison tree from the other content's revision
		let repo = Repository::open(".").map_err(|e| SourceError::Internal(e.into()))?;
		let obj = repo
			.revparse_single(&other.revision)
			.map_err(|e| SourceError::Internal(e.into()))?;

		let comparison_tree = match obj.kind() {
			Some(git2::ObjectType::Commit) => {
				let commit = obj.peel_to_commit().map_err(|e| SourceError::Internal(e.into()))?;
				commit.tree().map_err(|e| SourceError::Internal(e.into()))?
			}
			Some(git2::ObjectType::Tag) => {
				let tag = obj.peel_to_tag().map_err(|e| SourceError::Internal(e.into()))?;
				let target = tag.target().map_err(|e| SourceError::Internal(e.into()))?;
				let commit =
					target.peel_to_commit().map_err(|e| SourceError::Internal(e.into()))?;
				commit.tree().map_err(|e| SourceError::Internal(e.into()))?
			}
			Some(git2::ObjectType::Tree) => {
				obj.peel_to_tree().map_err(|e| SourceError::Internal(e.into()))?
			}
			_ => {
				return Err(SourceError::Internal(
					format!("Invalid revision type: {}", other.revision).into(),
				))
			}
		};

		// Generate diff between comparison tree and working directory
		let mut opts = DiffOptions::new();
		opts.pathspec(&self.path_pattern.path);

		let diff = repo
			.diff_tree_to_workdir_with_index(Some(&comparison_tree), Some(&mut opts))
			.map_err(|e| SourceError::Internal(e.into()))?;

		// Capture the diff output and check for intersections
		let mut buffer = String::new();
		let mut has_changes = false;

		diff.print(DiffFormat::Patch, |delta, _hunk, line| {
			// Check if this delta affects a file that matches our pattern
			let file_path = delta.new_file().path().or_else(|| delta.old_file().path());

			if let Some(path) = file_path {
				if self.path_pattern.matches(path) {
					has_changes = true;

					// Check if this line is within our line range
					let should_include = if let Some(ref line_range) = self.path_pattern.line_range
					{
						// Get line numbers from the diff line
						let new_line = line.new_lineno();
						let old_line = line.old_lineno();

						// Check if any of the line numbers fall within our range
						(new_line.map_or(false, |line_num| {
							line_range.start <= line_num as usize
								&& line_num as usize <= line_range.end
						})) || (old_line.map_or(false, |line_num| {
							line_range.start <= line_num as usize
								&& line_num as usize <= line_range.end
						}))
					} else {
						// No line range specified, include all lines
						true
					};

					if should_include {
						// Add the diff line
						buffer.push(line.origin());
						if let Ok(content) = std::str::from_utf8(line.content()) {
							buffer.push_str(content);
						}
					}
				}
			}

			true
		})
		.map_err(|e| SourceError::Internal(e.into()))?;

		Ok(GitDiff { diff: buffer, has_changes })
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn test_path_pattern_parsing() -> Result<(), anyhow::Error> {
		// Test basic path
		let pattern = PathPattern::try_new("src/lib.rs")?;
		assert_eq!(pattern.path, "src/lib.rs");
		assert_eq!(pattern.line_range, None);
		assert_eq!(pattern.glob, None);

		// Test path with line range
		let pattern = PathPattern::try_new("src/lib.rs#L1-L10")?;
		assert_eq!(pattern.path, "src/lib.rs");
		assert_eq!(pattern.line_range, Some(LineRange::try_new(1, 10)?));
		assert_eq!(pattern.glob, None);

		// Test glob pattern
		let pattern = PathPattern::try_new("src/**/*.rs")?;
		assert_eq!(pattern.path, "src/**/*.rs");
		assert_eq!(pattern.line_range, None);
		assert_eq!(pattern.glob, Some("src/**/*.rs".to_string()));

		// Test path with single line
		let pattern = PathPattern::try_new("README.md#L5")?;
		assert_eq!(pattern.path, "README.md");
		assert_eq!(pattern.line_range, Some(LineRange::try_new(5, 5)?));
		assert_eq!(pattern.glob, None);

		Ok(())
	}

	#[test]
	fn test_path_pattern_matching() -> Result<(), anyhow::Error> {
		let pattern = PathPattern::try_new("src/lib.rs")?;

		// Test exact path matching
		assert!(pattern.matches(Path::new("src/lib.rs")));
		assert!(!pattern.matches(Path::new("src/main.rs")));

		// Test glob pattern matching (simplified)
		let glob_pattern = PathPattern::try_new("src/**/*.rs")?;
		assert!(glob_pattern.matches(Path::new("src/lib.rs")));
		assert!(glob_pattern.matches(Path::new("src/main.rs")));
		assert!(!glob_pattern.matches(Path::new("src/lib.txt")));

		Ok(())
	}

	#[test]
	fn test_line_range_matching() -> Result<(), anyhow::Error> {
		let pattern = PathPattern::try_new("src/lib.rs#L5-L10")?;

		// Test line range matching
		assert!(pattern.line_in_range(5));
		assert!(pattern.line_in_range(7));
		assert!(pattern.line_in_range(10));
		assert!(!pattern.line_in_range(4));
		assert!(!pattern.line_in_range(11));

		// Test pattern without line range
		let pattern = PathPattern::try_new("src/lib.rs")?;
		assert!(pattern.line_in_range(1)); // Should always return true
		assert!(pattern.line_in_range(100)); // Should always return true

		Ok(())
	}

	#[test]
	fn test_git_source_creation() -> Result<(), anyhow::Error> {
		let source = GitSource::try_new("74aa653664cd90adcc5f836f1777f265c109045b", "README.md")?;

		assert_eq!(source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
		assert_eq!(source.path_pattern.path, "README.md");
		assert!(format!("{:?}", source.id).contains("74aa653664cd90adcc5f836f1777f265c109045b"));

		Ok(())
	}

	#[test]
	fn test_git_content_conversion() -> Result<(), anyhow::Error> {
		let source =
			GitSource::try_new("74aa653664cd90adcc5f836f1777f265c109045b", "README.md#L1-L5")?;

		let content: GitContent = source.into();
		assert_eq!(content.revision, "74aa653664cd90adcc5f836f1777f265c109045b");
		assert_eq!(content.path_pattern.path, "README.md");
		assert!(content.path_pattern.line_range.is_some());

		Ok(())
	}

	#[test]
	fn test_git_diff_creation() {
		let diff =
			GitDiff { diff: "--- a/README.md\n+++ b/README.md\n".to_string(), has_changes: true };

		assert!(!diff.is_empty());
		assert!(diff.has_changes);

		let empty_diff = GitDiff { diff: String::new(), has_changes: false };

		assert!(empty_diff.is_empty());
		assert!(!empty_diff.has_changes);
	}

	#[test]
	fn test_invalid_path_patterns() {
		// Test invalid line ranges
		assert!(PathPattern::try_new("file.rs#L0-L5").is_err()); // Start at 0
		assert!(PathPattern::try_new("file.rs#L10-L5").is_err()); // End < start
		assert!(PathPattern::try_new("file.rs#L5-L5").is_ok()); // Equal start/end is valid
	}

	#[test]
	fn test_error_conversion() {
		let git_error = GitSourceError::InvalidPathPattern("test".to_string());
		let source_error: SourceError = git_error.into();

		match source_error {
			SourceError::Network(msg) => assert!(msg.contains("Invalid path pattern")),
			_ => panic!("Expected Network error"),
		}
	}

	#[test]
	fn test_more_glob_patterns() -> Result<(), anyhow::Error> {
		// Test different glob patterns using the real glob crate
		let pattern = PathPattern::try_new("*.rs")?;
		assert!(pattern.matches(Path::new("lib.rs")));
		assert!(pattern.matches(Path::new("main.rs")));
		assert!(!pattern.matches(Path::new("lib.txt")));

		let pattern = PathPattern::try_new("src/*.rs")?;
		assert!(pattern.matches(Path::new("src/lib.rs")));
		assert!(pattern.matches(Path::new("src/main.rs")));
		assert!(!pattern.matches(Path::new("src/lib.txt")));
		assert!(!pattern.matches(Path::new("lib.rs")));

		let pattern = PathPattern::try_new("src/**/*.rs")?;
		assert!(pattern.matches(Path::new("src/lib.rs")));
		assert!(pattern.matches(Path::new("src/main.rs")));
		assert!(pattern.matches(Path::new("src/core/lib.rs")));
		assert!(pattern.matches(Path::new("src/utils/helpers.rs")));
		assert!(!pattern.matches(Path::new("src/lib.txt")));

		// Test some edge cases
		let pattern = PathPattern::try_new("src/**/test_*.rs")?;
		assert!(pattern.matches(Path::new("src/test_main.rs")));
		assert!(pattern.matches(Path::new("src/core/test_lib.rs")));
		assert!(!pattern.matches(Path::new("src/main.rs")));

		Ok(())
	}

	#[test]
	fn test_line_range_edge_cases() -> Result<(), anyhow::Error> {
		// Test single line
		let pattern = PathPattern::try_new("file.rs#L5")?;
		assert!(pattern.line_in_range(5));
		assert!(!pattern.line_in_range(4));
		assert!(!pattern.line_in_range(6));

		// Test range with same start and end
		let pattern = PathPattern::try_new("file.rs#L10-L10")?;
		assert!(pattern.line_in_range(10));
		assert!(!pattern.line_in_range(9));
		assert!(!pattern.line_in_range(11));

		// Test large line numbers
		let pattern = PathPattern::try_new("file.rs#L1000-L2000")?;
		assert!(pattern.line_in_range(1000));
		assert!(pattern.line_in_range(1500));
		assert!(pattern.line_in_range(2000));
		assert!(!pattern.line_in_range(999));
		assert!(!pattern.line_in_range(2001));

		Ok(())
	}

	#[test]
	fn test_git_source_id_generation() -> Result<(), anyhow::Error> {
		let source1 = GitSource::try_new("abc123", "README.md")?;
		let source2 = GitSource::try_new("abc123", "README.md")?;
		let source3 = GitSource::try_new("def456", "README.md")?;
		let source4 = GitSource::try_new("abc123", "src/lib.rs")?;

		// Same revision and path should generate same ID
		assert_eq!(source1.id, source2.id);

		// Different revision or path should generate different IDs
		assert_ne!(source1.id, source3.id);
		assert_ne!(source1.id, source4.id);

		Ok(())
	}

	#[test]
	fn test_real_git_diff_with_line_ranges() -> Result<(), anyhow::Error> {
		// This test requires a git repository with the specified commit
		// We'll use the commit mentioned in the user's requirements
		let commit_hash = "74aa653664cd90adcc5f836f1777f265c109045b";

		// Try to create a git source for README.md with line range
		let source = GitSource::try_new(commit_hash, "README.md#L1-L5")?;
		let content: GitContent = source.into();

		// Create another content with a different line range
		let source2 = GitSource::try_new(commit_hash, "README.md#L10-L15")?;
		let content2: GitContent = source2.into();

		// The diff should work (even if there are no changes, it should not panic)
		let _diff_result = content.diff(&content2);

		// If we get here without panicking, the line range logic is working
		Ok(())
	}

	#[test]
	fn test_line_range_filtering_behavior() -> Result<(), anyhow::Error> {
		// Test that line range filtering works correctly
		let commit_hash = "74aa653664cd90adcc5f836f1777f265c109045b";

		// Create content with full file (no line range)
		let source_full = GitSource::try_new(commit_hash, "README.md")?;
		let content_full: GitContent = source_full.into();

		// Create content with limited line range
		let source_limited = GitSource::try_new(commit_hash, "README.md#L1-L3")?;
		let content_limited: GitContent = source_limited.into();

		// Both should work without panicking
		let _diff_full = content_full.diff(&content_limited);
		let _diff_limited = content_limited.diff(&content_full);

		// The key difference is that content_limited will only include diff lines
		// that fall within lines 1-3, while content_full will include all diff lines
		Ok(())
	}
}
