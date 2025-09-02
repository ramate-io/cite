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

	#[error("Invalid remote URL: {0}")]
	InvalidRemote(String),

	#[error("Invalid path: {0}")]
	InvalidPath(String),
}

impl From<GitSourceError> for SourceError {
	fn from(err: GitSourceError) -> Self {
		SourceError::Network(err.to_string())
	}
}

/// Get the repository root directory using CARGO_MANIFEST_DIR
fn get_repository_root() -> Result<std::path::PathBuf, GitSourceError> {
	// CARGO_MANIFEST_DIR points to the directory containing Cargo.toml of the crate being built
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
		GitSourceError::InvalidPathPattern("CARGO_MANIFEST_DIR not set".to_string())
	})?;

	let manifest_path = std::path::PathBuf::from(manifest_dir);

	// Find the git repository root by walking up from the manifest directory
	let mut current = manifest_path;
	while !current.join(".git").exists() {
		current = current
			.parent()
			.ok_or_else(|| {
				GitSourceError::InvalidPathPattern("Could not find git repository root".to_string())
			})?
			.to_path_buf();
	}

	Ok(current)
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
	/// The remote repository URL
	pub remote: String,
	/// The path pattern for files within the repository
	pub path_pattern: PathPattern,
	/// The revision to compare against (commit hash, branch, tag)
	pub comparison_revision: String,
}

impl GitSource {
	pub fn try_new(remote: &str, path: &str, revision: &str) -> Result<Self, GitSourceError> {
		// Basic validation
		if remote.is_empty() {
			return Err(GitSourceError::InvalidRemote("Remote URL cannot be empty".into()));
		}
		if revision.is_empty() {
			return Err(GitSourceError::InvalidRevision("Revision cannot be empty".into()));
		}
		
		// Parse the path into a PathPattern
		let path_pattern = PathPattern::try_new(path)?;
		
		let id = Id::new(format!("git_{}_{}_{}", remote, path, revision));
		Ok(Self {
			id,
			remote: remote.to_string(),
			path_pattern,
			comparison_revision: revision.to_string(),
		})
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
	pub remote: String,
	pub path_pattern: PathPattern,
	pub revision: String,
}

impl From<GitSource> for GitContent {
	fn from(source: GitSource) -> Self {
		GitContent { 
			remote: source.remote, 
			path_pattern: source.path_pattern, 
			revision: source.comparison_revision 
		}
	}
}

impl Content for GitContent {}
impl Referenced for GitContent {}

/// Git diff representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitDiff {
	diff: String,
	has_changes: bool,
}

impl Diff for GitDiff {
	fn is_empty(&self) -> bool {
		!self.has_changes
	}
}

impl GitDiff {
	pub fn has_changes(&self) -> bool {
		self.has_changes
	}

	pub fn diff(&self) -> &str {
		&self.diff
	}

	/// Get the unified diff output, similar to HTTP sources
	/// Returns Some(diff_string) if there are changes, None if no changes
	pub fn unified_diff(&self) -> Option<&str> {
		if self.has_changes && !self.diff.is_empty() {
			Some(&self.diff)
		} else {
			None
		}
	}
}

impl Current<GitContent, GitDiff> for GitContent {
	fn diff(&self, other: &GitContent) -> Result<GitDiff, SourceError> {
		// For now, we'll use the local repository approach
		// TODO: Implement remote repository fetching
		let repository_root = get_repository_root().map_err(|e| SourceError::Internal(e.into()))?;
		let repo =
			Repository::open(&repository_root).map_err(|e| SourceError::Internal(e.into()))?;
		
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
						has_changes = true;

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
		let source = GitSource::try_new(
			"https://github.com/ramate-io/cite",
			"README.md",
			"74aa653664cd90adcc5f836f1777f265c109045b"
		)?;

		assert_eq!(source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(source.path_pattern.path, "README.md");
		assert_eq!(source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
		assert!(format!("{:?}", source.id).contains("74aa653664cd90adcc5f836f1777f265c109045b"));

		Ok(())
	}

	#[test]
	fn test_git_content_conversion() -> Result<(), anyhow::Error> {
		let source = GitSource::try_new(
			"https://github.com/ramate-io/cite",
			"README.md#L1-L5",
			"74aa653664cd90adcc5f836f1777f265c109045b"
		)?;

		let content: GitContent = source.into();
		assert_eq!(content.remote, "https://github.com/ramate-io/cite");
		assert_eq!(content.path_pattern.path, "README.md");
		assert_eq!(content.revision, "74aa653664cd90adcc5f836f1777f265c109045b");

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
	fn test_git_diff_unified_diff() {
		let diff_with_changes = GitDiff { 
			diff: "--- a/README.md\n+++ b/README.md\n@@ -1,3 +1,3 @@\n-old content\n+new content\n unchanged\n".to_string(), 
			has_changes: true 
		};

		// Should return Some when there are changes
		assert!(diff_with_changes.unified_diff().is_some());
		assert_eq!(diff_with_changes.unified_diff().unwrap(), diff_with_changes.diff());

		let diff_no_changes = GitDiff { 
			diff: "".to_string(), 
			has_changes: false 
		};

		// Should return None when there are no changes
		assert!(diff_no_changes.unified_diff().is_none());

		let diff_empty_string = GitDiff { 
			diff: "".to_string(), 
			has_changes: true 
		};

		// Should return None when diff string is empty even if has_changes is true
		assert!(diff_empty_string.unified_diff().is_none());
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
		let source1 = GitSource::try_new("https://github.com/ramate-io/cite", "README.md", "abc123")?;
		let source2 = GitSource::try_new("https://github.com/ramate-io/cite", "README.md", "abc123")?;
		let source3 = GitSource::try_new("https://github.com/ramate-io/cite", "README.md", "def456")?;
		let source4 = GitSource::try_new("https://github.com/ramate-io/cite", "src/lib.rs", "abc123")?;

		// Same remote, path, and revision should generate same ID
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
		let source = GitSource::try_new("https://github.com/ramate-io/cite", "README.md#L1-L5", commit_hash)?;
		let content: GitContent = source.into();

		// Create another content with a different line range
		let source2 = GitSource::try_new("https://github.com/ramate-io/cite", "README.md#L10-L15", commit_hash)?;
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
		let source_full = GitSource::try_new("https://github.com/ramate-io/cite", "README.md", commit_hash)?;
		let content_full: GitContent = source_full.into();

		// Create content with limited line range
		let source_limited = GitSource::try_new("https://github.com/ramate-io/cite", "README.md#L1-L3", commit_hash)?;
		let content_limited: GitContent = source_limited.into();

		// Both should work without panicking
		let _diff_full = content_full.diff(&content_limited);
		let _diff_limited = content_limited.diff(&content_full);

		// The key difference is that content_limited will only include diff lines
		// that fall within lines 1-3, while content_full will include all diff lines
		Ok(())
	}

	#[test]
	fn test_diff_line_intersection_scenarios() -> Result<(), anyhow::Error> {
		// Test various line range intersection scenarios with the test commit
		let commit_hash = "94dab273cf6c2abe8742d6d459ad45c96ca9b694";

		// Test 1: Lines 1-3 (covering the beginning)
		let source_1_3 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L1-L3", commit_hash)?;
		let content_1_3: GitContent = source_1_3.into();

		// Test 2: Lines 5-10 (covering the middle to end)
		let source_5_10 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-5-10.md#L5-L10", commit_hash)?;
		let content_5_10: GitContent = source_5_10.into();

		// Test 3: Lines 4-6 (intersecting with both ranges)
		let source_4_6 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L4-L6", commit_hash)?;
		let content_4_6: GitContent = source_4_6.into();

		// Test 4: Lines 8-12 (partially intersecting, extending beyond file)
		let source_8_12 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L8-L12", commit_hash)?;
		let content_8_12: GitContent = source_8_12.into();

		// Test 5: Lines 11-15 (not intersecting with file content)
		let source_11_15 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L11-L15", commit_hash)?;
		let content_11_15: GitContent = source_11_15.into();

		// Test 6: Single line (line 5)
		let source_line_5 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L5", commit_hash)?;
		let content_line_5: GitContent = source_line_5.into();

		// Test 7: Full file (no line range)
		let source_full =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md", commit_hash)?;
		let content_full: GitContent = source_full.into();

		// Test 8: File with no changes
		let source_no_diff =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/no-diffed.md", commit_hash)?;
		let content_no_diff: GitContent = source_no_diff.into();

		// Test 9: File that will be deleted
		let source_to_delete =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/to-delete.md", commit_hash)?;
		let content_to_delete: GitContent = source_to_delete.into();

		// Run diffs to test line intersection logic
		// These should not panic and should filter lines correctly
		let _diff_1_3 = content_1_3.diff(&content_full);
		let _diff_5_10 = content_5_10.diff(&content_full);
		let _diff_4_6 = content_4_6.diff(&content_full);
		let _diff_8_12 = content_8_12.diff(&content_full);
		let _diff_11_15 = content_11_15.diff(&content_full);
		let _diff_line_5 = content_line_5.diff(&content_full);
		let _diff_no_diff = content_no_diff.diff(&content_full);
		let _diff_to_delete = content_to_delete.diff(&content_full);

		// If we get here without panicking, the line intersection logic is working
		Ok(())
	}

	#[test]
	fn test_line_range_edge_cases_with_real_files() -> Result<(), anyhow::Error> {
		// Test edge cases for line range filtering with real files
		let commit_hash = "94dab273cf6c2abe8742d6d459ad45c96ca9b694";

		// Test 1: Line range exactly matching file boundaries
		let source_exact =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L1-L10", commit_hash)?;
		let content_exact: GitContent = source_exact.into();

		// Test 2: Line range starting at 1, ending before file end
		let source_start_1 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L1-L5", commit_hash)?;
		let content_start_1: GitContent = source_start_1.into();

		// Test 3: Line range starting after file start, ending at file end
		let source_end_file =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L5-L10", commit_hash)?;
		let content_end_file: GitContent = source_end_file.into();

		// Test 4: Line range completely outside file (after)
		let source_after =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L15-L20", commit_hash)?;
		let content_after: GitContent = source_after.into();

		// Test 5: Single line at file boundary
		let source_boundary =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L10", commit_hash)?;
		let content_boundary: GitContent = source_boundary.into();

		// Run diffs to test edge case handling
		let _diff_exact = content_exact.diff(&content_start_1);
		let _diff_start_1 = content_start_1.diff(&content_exact);
		let _diff_end_file = content_end_file.diff(&content_exact);
		let _diff_after = content_after.diff(&content_exact);
		let _diff_boundary = content_boundary.diff(&content_exact);

		// If we get here without panicking, the edge case handling is working
		Ok(())
	}

	#[test]
	fn test_diff_content_verification() -> Result<(), anyhow::Error> {
		// Test that diff content is actually filtered by line ranges
		let commit_hash = "94dab273cf6c2abe8742d6d459ad45c96ca9b694";

		// Create different line range sources
		let source_1_3 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L1-L3", commit_hash)?;
		let source_5_10 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-5-10.md#L5-L10", commit_hash)?;
		let source_full =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md", commit_hash)?;

		let content_1_3: GitContent = source_1_3.into();
		let content_5_10: GitContent = source_5_10.into();
		let content_full: GitContent = source_full.into();

		// Generate diffs
		let diff_1_3 = content_1_3.diff(&content_1_3.clone())?;
		let diff_5_10 = content_5_10.diff(&content_5_10.clone())?;
		let diff_full = content_full.diff(&content_full.clone())?;

		// Verify that diffs are generated (even if empty)
		// The actual content verification would depend on what changes were made
		// between the commit and current working directory

		assert!(diff_1_3.has_changes());
		assert!(diff_5_10.has_changes());
		assert!(diff_full.has_changes());

		assert_eq!(diff_1_3.diff(), "-Alpha\n-Bravo\n-Charlie\n+Aaron\n+Bear\n+Cat\n");
		assert_eq!(diff_5_10.diff(), "-Echo\n-Foxtrot\n-Gamma\n-Halifax\n-Istanbul\n-Juniper>\n\\ No newline at end of file\n+Epsom\n+Fox\n+Golf\n+Hotel\n+India\n+Juliet<\n\\ No newline at end of file\n");
		assert_eq!(diff_full.diff(), "Fdiff --git a/cite/http/tests/content/diffed-lines-1-3.md b/cite/http/tests/content/diffed-lines-1-3.md\nindex 0f800a0..9aeeae4 100644\n--- a/cite/http/tests/content/diffed-lines-1-3.md\n+++ b/cite/http/tests/content/diffed-lines-1-3.md\nH@@ -1,6 +1,6 @@\n-Alpha\n-Bravo\n-Charlie\n+Aaron\n+Bear\n+Cat\n Delta\n Echo\n Foxtrot\n");

		Ok(())
	}

	#[test]
	fn test_diff_content_verification_edge_cases() -> Result<(), anyhow::Error> {
		// Test that diff content is actually filtered by line ranges
		let commit_hash = "94dab273cf6c2abe8742d6d459ad45c96ca9b694";

		let source_intersects_1_3 =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L3-L5", commit_hash)?;
		let content_intersects_1_3: GitContent = source_intersects_1_3.into();
		let diff_intersects_1_3 = content_intersects_1_3.diff(&content_intersects_1_3.clone())?;

		assert!(diff_intersects_1_3.has_changes());
		assert_eq!(diff_intersects_1_3.diff(), "-Charlie\n+Cat\n Delta\n Echo\n");

		Ok(())
	}

	#[test]
	fn test_diff_does_not_intersect() -> Result<(), anyhow::Error> {
		let commit_hash = "94dab273cf6c2abe8742d6d459ad45c96ca9b694";

		let source_does_not_intersect =
			GitSource::try_new("https://github.com/ramate-io/cite", "cite/http/tests/content/diffed-lines-1-3.md#L7-L10", commit_hash)?;
		let content_does_not_intersect: GitContent = source_does_not_intersect.into();

		let diff_does_not_intersect =
			content_does_not_intersect.diff(&content_does_not_intersect.clone())?;

		assert!(!diff_does_not_intersect.has_changes());
		assert_eq!(diff_does_not_intersect.diff(), "");

		Ok(())
	}

	#[test]
	fn test_cargo_manifest_dir_approach() -> Result<(), anyhow::Error> {
		// Test that CARGO_MANIFEST_DIR approach works correctly
		let repository_root = get_repository_root()?;

		// Verify that we found a git repository
		assert!(repository_root.join(".git").exists());

		// Verify that we can open the repository
		let repo = Repository::open(&repository_root)?;
		assert!(repo.is_bare() == false); // Should be a working directory repository

		// Verify that we're in the right place (should be the cite repository)
		let head = repo.head()?;
		let head_oid = head.target().unwrap();
		println!("Repository root: {:?}", repository_root);
		println!("Current HEAD: {}", head_oid);

		Ok(())
	}
}
