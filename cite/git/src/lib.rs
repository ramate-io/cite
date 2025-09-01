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

	pub fn try_from_string(string: &str) -> Result<Self, GitSourceError> {
		let path_pattern = PathPattern::try_new(string)?;
		Ok(path_pattern)
	}

	pub fn matches(&self, _path: &Path, _line: Option<usize>) -> bool {
		return true;
	}
}

/// Git source configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitSource {
	pub id: Id,
	pub remote: String,
	pub comparison_revision: String,
	pub paths: PathPattern,
}

impl GitSource {
	pub fn try_new(
		remote: &str,
		comparison_revision: &str,
		pattern: &str,
	) -> Result<Self, GitSourceError> {
		let path_pattern = PathPattern::try_new(pattern)?;
		Ok(Self {
			id: Id::new(format!("{}/{}", remote, comparison_revision)),
			remote: remote.to_string(),
			comparison_revision: comparison_revision.to_string(),
			paths: path_pattern,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitContent {
	revision: String,
	path: PathPattern,
}

impl From<GitSource> for GitContent {
	fn from(source: GitSource) -> Self {
		GitContent { revision: source.comparison_revision, path: source.paths }
	}
}

impl Content for GitContent {}
impl Referenced for GitContent {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitDiff {
	diff: String,
}

impl Diff for GitDiff {
	fn is_empty(&self) -> bool {
		self.diff.is_empty()
	}
}

impl Current<GitContent, GitDiff> for GitContent {
	fn diff(&self, other: &GitContent) -> Result<GitDiff, SourceError> {
		// use the repo to form the comparison
		let repo = Repository::open(".").map_err(|e| SourceError::Internal(e.into()))?;
		let obj = repo
			.revparse_single(other.revision.as_str())
			.map_err(|e| SourceError::Internal(e.into()))?;
		let commit = obj.peel_to_commit().map_err(|e| SourceError::Internal(e.into()))?;
		let comparison_tree = commit.tree().map_err(|e| SourceError::Internal(e.into()))?;

		let mut opts = DiffOptions::new();
		let diff = repo
			.diff_tree_to_workdir_with_index(Some(&comparison_tree), Some(&mut opts))
			.map_err(|e| SourceError::Internal(e.into()))?;

		// Capture the entire diff into a string, check if it intersects with the match
		let mut buffer = String::new();
		let mut intersects = false;
		let mut errors = Vec::new();
		diff.print(DiffFormat::Patch, |delta, _hunk, line| {
			let current_file = delta.new_file();
			let current_file_path = if let Some(path) = current_file.path() {
				path
			} else {
				errors.push(format!("Current file path not found: {:?}", current_file.path()));
				return true;
			};

			let comparison_file = delta.old_file();
			let comparison_file_path = if let Some(path) = comparison_file.path() {
				path
			} else {
				errors
					.push(format!("Comparison file path not found: {:?}", comparison_file.path()));
				return true;
			};

			let _current_lineno = line.new_lineno();
			let _comparison_lineno = line.old_lineno();

			let line_range = None; // for now we are not going to match on line range

			if self.path.matches(current_file_path, line_range)
				|| self.path.matches(comparison_file_path, line_range)
			{
				intersects = true;
			}

			// get the sign of the line diff
			buffer.push(line.origin());
			let content = line.content();
			if let Ok(content) = std::str::from_utf8(content) {
				buffer.push_str(content);
			}

			true
		})
		.map_err(|e| SourceError::Internal(e.into()))?;

		let diff = if intersects { buffer } else { String::new() };
		Ok(GitDiff { diff })
	}
}
