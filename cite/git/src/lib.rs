pub mod line_range;
pub use line_range::LineRange;

use cite_cache::{CacheError, CacheableCurrent, CacheableReferenced};
use cite_core::{Comparison, Content, Current, Diff, Id, Referenced, Source, SourceError};
use git2::{Error as GitError, ObjectType, Repository, Tree};
use glob::Pattern;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Error types for git operations
#[derive(Error, Debug)]
pub enum GitSourceError {
	#[error("Git operation failed: {0}")]
	Git(#[from] GitError),

	#[error("Invalid path pattern: {0}")]
	InvalidPathPattern(String),

	#[error("Path not found in repository: {0}")]
	PathNotFound(String),

	#[error("Invalid revision: {0}")]
	InvalidRevision(String),

	#[error("Repository clone failed: {0}")]
	CloneFailed(String),

	#[error("Cache error: {0}")]
	Cache(#[from] CacheError),
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
			if let Ok(pattern) = Pattern::new(glob_pattern) {
				return pattern.matches_path(path);
			}
		} else {
			// Exact path match
			return path.to_string_lossy() == self.path;
		}
		false
	}
}

/// Git source configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitSource {
	pub remote: String,
	pub default_branch: String,
	pub revision: String,
	pub paths: Vec<PathPattern>,
	pub cache_dir: PathBuf,
}

impl GitSource {
	pub fn new(
		remote: &str,
		default_branch: &str,
		revision: &str,
		paths: Vec<&str>,
	) -> Result<Self, GitSourceError> {
		let path_patterns: Result<Vec<_>, _> =
			paths.iter().map(|p| PathPattern::try_new(p)).collect();

		let path_patterns = path_patterns?;

		// Create cache directory based on remote URL
		let cache_dir = Self::remote_to_cache_dir(remote);

		Ok(Self {
			remote: remote.to_string(),
			default_branch: default_branch.to_string(),
			revision: revision.to_string(),
			paths: path_patterns,
			cache_dir,
		})
	}

	/// Convert remote URL to a safe cache directory name
	fn remote_to_cache_dir(remote: &str) -> PathBuf {
		let sanitized = remote
			.chars()
			.map(|c| match c {
				'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
				_ => '_',
			})
			.collect::<String>();

		PathBuf::from(format!("git_{}", sanitized))
	}

	/// Get or clone the repository
	pub fn get_repository(&self) -> Result<Repository, GitSourceError> {
		if self.cache_dir.exists() {
			// Try to open existing repository
			if let Ok(repo) = Repository::open(&self.cache_dir) {
				// Update the repository
				if let Err(e) = self.update_repository(&repo) {
					eprintln!("Warning: Failed to update repository: {}", e);
				}
				return Ok(repo);
			}
		}

		// Clone the repository
		self.clone_repository()
	}

	/// Clone the repository to cache directory
	fn clone_repository(&self) -> Result<Repository, GitSourceError> {
		// Create parent directory if it doesn't exist
		if let Some(parent) = self.cache_dir.parent() {
			std::fs::create_dir_all(parent).map_err(|e| {
				GitSourceError::CloneFailed(format!("Failed to create cache directory: {}", e))
			})?;
		}

		let repo = Repository::clone(&self.remote, &self.cache_dir).map_err(|e| {
			GitSourceError::CloneFailed(format!("Failed to clone repository: {}", e))
		})?;

		Ok(repo)
	}

	/// Update existing repository
	fn update_repository(&self, repo: &Repository) -> Result<(), GitSourceError> {
		// Fetch latest changes
		let mut remote = repo
			.find_remote("origin")
			.map_err(|_| GitSourceError::CloneFailed("No origin remote found".to_string()))?;

		remote
			.fetch(&[&self.default_branch], None, None)
			.map_err(|e| GitSourceError::Git(e))?;

		Ok(())
	}

	/// Get the tree for a specific revision
	pub fn get_tree<'a>(
		&self,
		repo: &'a Repository,
		revision: &str,
	) -> Result<Tree<'a>, GitSourceError> {
		let obj = repo
			.revparse_single(revision)
			.map_err(|_| GitSourceError::InvalidRevision(revision.to_string()))?;

		match obj.kind() {
			Some(ObjectType::Commit) => {
				let commit = obj.peel_to_commit().map_err(|e| GitSourceError::Git(e))?;
				let tree = commit.tree().map_err(|e| GitSourceError::Git(e))?;
				Ok(tree)
			}
			Some(ObjectType::Tag) => {
				let tag = obj.peel_to_tag().map_err(|e| GitSourceError::Git(e))?;
				let target = tag.target().map_err(|e| GitSourceError::Git(e))?;
				let commit = target.peel_to_commit().map_err(|e| GitSourceError::Git(e))?;
				let tree = commit.tree().map_err(|e| GitSourceError::Git(e))?;
				Ok(tree)
			}
			Some(ObjectType::Tree) => {
				let tree = obj.peel_to_tree().map_err(|e| GitSourceError::Git(e))?;
				Ok(tree)
			}
			_ => Err(GitSourceError::InvalidRevision(format!(
				"Revision {} is not a commit, tag, or tree",
				revision
			))),
		}
	}

	/// Extract content from a file at a specific revision
	pub fn extract_file_content(
		&self,
		repo: &Repository,
		tree: &Tree,
		path: &str,
		line_range: Option<&LineRange>,
	) -> Result<String, GitSourceError> {
		let entry = tree
			.get_path(Path::new(path))
			.map_err(|_| GitSourceError::PathNotFound(path.to_string()))?;

		let blob = entry
			.to_object(repo)
			.map_err(|e| GitSourceError::Git(e))?
			.peel_to_blob()
			.map_err(|e| GitSourceError::Git(e))?;

		let content = String::from_utf8(blob.content().to_vec())
			.map_err(|_| GitSourceError::PathNotFound("Invalid UTF-8 content".to_string()))?;

		// Apply line range if specified
		if let Some(range) = line_range {
			let lines: Vec<&str> = content.lines().collect();
			if range.start > lines.len() || range.end > lines.len() {
				return Err(GitSourceError::InvalidPathPattern(format!(
					"Line range {}:{} exceeds file length {}",
					range.start,
					range.end,
					lines.len()
				)));
			}

			let selected_lines = &lines[range.start - 1..range.end];
			Ok(selected_lines.join("\n"))
		} else {
			Ok(content)
		}
	}

	/// Get all matching files and their content for a revision
	pub fn get_files_content(
		&self,
		repo: &Repository,
		tree: &Tree,
	) -> Result<HashMap<String, String>, GitSourceError> {
		let mut result = HashMap::new();

		for pattern in &self.paths {
			if let Some(ref glob_pattern) = pattern.glob {
				// Handle glob patterns
				let pattern_obj = Pattern::new(glob_pattern).map_err(|e| {
					GitSourceError::InvalidPathPattern(format!(
						"Invalid glob pattern '{}': {}",
						glob_pattern, e
					))
				})?;

				// Walk the tree to find matching files
				self.walk_tree_for_glob(repo, tree, &pattern_obj, &mut result)?;
			} else {
				// Handle exact path
				let content = self.extract_file_content(
					repo,
					tree,
					&pattern.path,
					pattern.line_range.as_ref(),
				)?;
				result.insert(pattern.path.clone(), content);
			}
		}

		Ok(result)
	}

	/// Walk tree to find files matching a glob pattern
	fn walk_tree_for_glob(
		&self,
		repo: &Repository,
		tree: &Tree,
		pattern: &Pattern,
		result: &mut HashMap<String, String>,
	) -> Result<(), GitSourceError> {
		let mut stack = vec![(tree.id(), PathBuf::new())];

		while let Some((tree_id, current_path)) = stack.pop() {
			let current_tree = repo.find_tree(tree_id).map_err(|e| GitSourceError::Git(e))?;

			for entry in current_tree.iter() {
				let entry_name = entry.name().unwrap_or("");
				let entry_path = current_path.join(entry_name);

				match entry.kind() {
					Some(ObjectType::Tree) => {
						stack.push((entry.id(), entry_path));
					}
					Some(ObjectType::Blob) => {
						if pattern.matches_path(&entry_path) {
							let path_str = entry_path.to_str().ok_or_else(|| {
								GitSourceError::InvalidPathPattern(format!(
									"Invalid path encoding: {:?}",
									entry_path
								))
							})?;
							let content = self.extract_file_content(
								repo,
								&current_tree,
								path_str,
								None, // No line range for glob patterns
							)?;
							result.insert(entry_path.to_string_lossy().to_string(), content);
						}
					}
					_ => {}
				}
			}
		}

		Ok(())
	}
}

/// Git content that was referenced at commit time
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencedGit {
	/// The extracted content that was referenced
	pub content: HashMap<String, String>,
	/// Metadata about the git source
	pub metadata: HashMap<String, String>,
	/// The git source configuration
	pub source: GitSource,
	/// The revision that was referenced
	pub revision: String,
}

impl Content for ReferencedGit {}
impl Referenced for ReferencedGit {}

impl CacheableReferenced for ReferencedGit {
	fn from_cached_buffer(buffer: Vec<u8>) -> Result<Self, CacheError> {
		serde_json::from_slice(&buffer).map_err(|e| CacheError::Deserialize(e.into()))
	}
}

/// Current git content fetched from the repository
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentGit {
	/// The extracted content currently available
	pub content: HashMap<String, String>,
	/// Current metadata (commit hash, timestamp, etc.)
	pub metadata: HashMap<String, String>,
	/// The git source configuration
	pub source: GitSource,
	/// The current revision being checked
	pub current_revision: String,
}

impl Content for CurrentGit {}

impl Current<ReferencedGit, GitDiff> for CurrentGit {
	fn diff(&self, referenced: &ReferencedGit) -> Result<GitDiff, SourceError> {
		let mut diff = GitDiff {
			content_changed: false,
			revision_changed: false,
			path_changes: HashMap::new(),
			unified_diffs: HashMap::new(),
		};

		// Check if revision changed
		if self.current_revision != referenced.revision {
			diff.revision_changed = true;
		}

		// Check content changes for each path
		let all_paths: std::collections::HashSet<_> =
			self.content.keys().chain(referenced.content.keys()).collect();

		for path in all_paths {
			let current_content = self.content.get(path).cloned().unwrap_or_default();
			let referenced_content = referenced.content.get(path).cloned().unwrap_or_default();

			if current_content != referenced_content {
				diff.content_changed = true;
				let path_change = PathChange {
					path: path.clone(),
					referenced_content: referenced_content.clone(),
					current_content: current_content.clone(),
				};
				diff.path_changes.insert(path.clone(), path_change);

				// Generate unified diff for this path
				let unified_diff =
					Self::generate_unified_diff(&referenced_content, &current_content, path);
				diff.unified_diffs.insert(path.clone(), unified_diff);
			}
		}

		Ok(diff)
	}
}

impl CacheableCurrent<ReferencedGit, GitDiff> for CurrentGit {
	fn to_cached_buffer(&self) -> Result<Vec<u8>, CacheError> {
		// Convert to ReferencedGit format for caching
		let referenced = ReferencedGit {
			content: self.content.clone(),
			metadata: self.metadata.clone(),
			source: self.source.clone(),
			revision: self.current_revision.clone(),
		};
		serde_json::to_vec(&referenced).map_err(|e| CacheError::Serialize(e.into()))
	}
}

/// Change information for a specific path
#[derive(Debug, Clone, PartialEq)]
pub struct PathChange {
	pub path: String,
	pub referenced_content: String,
	pub current_content: String,
}

/// Diff between referenced and current git content
#[derive(Debug, Clone, PartialEq)]
pub struct GitDiff {
	pub content_changed: bool,
	pub revision_changed: bool,
	pub path_changes: HashMap<String, PathChange>,
	pub unified_diffs: HashMap<String, String>,
}

impl GitDiff {
	/// Get unified diff for a specific path
	pub fn get_unified_diff(&self, path: &str) -> Option<&str> {
		self.unified_diffs.get(path).map(|s| s.as_str())
	}

	/// Get all changed paths
	pub fn changed_paths(&self) -> Vec<&String> {
		self.path_changes.keys().collect()
	}
}

impl Diff for GitDiff {
	fn is_empty(&self) -> bool {
		!self.content_changed && !self.revision_changed
	}
}

impl CurrentGit {
	/// Generate a git-style unified diff for a file
	fn generate_unified_diff(
		referenced_content: &str,
		current_content: &str,
		path: &str,
	) -> String {
		let diff = TextDiff::from_lines(referenced_content, current_content);
		let mut result = Vec::new();

		// Add diff header
		result.push(format!("--- a/{}\n", path));
		result.push(format!("+++ b/{}\n", path));

		for change in diff.iter_all_changes() {
			let sign = match change.tag() {
				ChangeTag::Delete => "-",
				ChangeTag::Insert => "+",
				ChangeTag::Equal => " ",
			};
			result.push(format!("{}{}", sign, change));
		}

		result.join("")
	}
}

/// Git match source for checking committed git references
#[derive(Clone)]
pub struct GitMatch {
	pub source: GitSource,
	pub cache_path: String,
	id: Id,
	cache: cite_cache::Cache,
	cache_behavior: cite_cache::CacheBehavior,
}

impl GitMatch {
	/// Create a new git match
	pub fn new(
		remote: &str,
		default_branch: &str,
		revision: &str,
		paths: Vec<&str>,
	) -> Result<Self, SourceError> {
		use cite_cache::CacheBuilder;

		let source = GitSource::new(remote, default_branch, revision, paths)?;
		let cache_path = format!(
			"git_{}_{}_{}",
			Self::remote_to_cache_key(remote),
			Self::revision_to_cache_key(revision),
			Self::paths_to_cache_key(&source.paths)
		);
		let id = Id::new(cache_path.clone());

		let cache_builder = CacheBuilder::default();
		let cache = cache_builder
			.build()
			.map_err(|e| SourceError::Network(format!("Failed to create cache: {}", e)))?;

		Ok(Self {
			source,
			cache_path,
			id,
			cache,
			cache_behavior: cite_cache::CacheBehavior::Enabled,
		})
	}

	/// Convert remote URL to a safe cache key
	fn remote_to_cache_key(remote: &str) -> String {
		remote
			.chars()
			.map(|c| match c {
				'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
				_ => '_',
			})
			.collect()
	}

	/// Convert revision to a safe cache key
	fn revision_to_cache_key(revision: &str) -> String {
		revision
			.chars()
			.map(|c| match c {
				'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
				_ => '_',
			})
			.collect()
	}

	/// Convert paths to a safe cache key
	fn paths_to_cache_key(paths: &[PathPattern]) -> String {
		let path_strings: Vec<String> = paths
			.iter()
			.map(|p| {
				let mut key = p.path.clone();
				if let Some(ref range) = p.line_range {
					key.push_str(&format!("#L{}-L{}", range.start, range.end));
				}
				key
			})
			.collect();

		let combined = path_strings.join("_");
		combined
			.chars()
			.map(|c| match c {
				'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
				_ => '_',
			})
			.collect()
	}

	/// Flushes the cache for this source
	pub fn flush_cache(&self) -> Result<(), SourceError> {
		match self.cache.delete(self.id()) {
			Ok(_) => Ok(()),
			Err(CacheError::CacheFileNotFound(_)) => Ok(()),
			Err(e) => Err(SourceError::Cache(format!("Failed to flush cache: {}", e))),
		}
	}
}

impl Source<ReferencedGit, CurrentGit, GitDiff> for GitMatch {
	fn id(&self) -> &Id {
		&self.id
	}

	fn get(&self) -> Result<Comparison<ReferencedGit, CurrentGit, GitDiff>, SourceError> {
		self.cache
			.get_source_with_cache(self, self.cache_behavior.clone())
			.map_err(|e| SourceError::Network(format!("Cache error: {}", e)))
	}

	fn get_referenced(&self) -> Result<ReferencedGit, SourceError> {
		// This provides fallback when no cache is available
		let current = self.get_current()?;

		Ok(ReferencedGit {
			content: current.content,
			metadata: current.metadata,
			source: self.source.clone(),
			revision: current.current_revision.clone(),
		})
	}

	fn get_current(&self) -> Result<CurrentGit, SourceError> {
		let repo = self.source.get_repository()?;
		let tree = self.source.get_tree(&repo, &self.source.revision)?;
		let content = self.source.get_files_content(&repo, &tree)?;

		let mut metadata = HashMap::new();
		metadata.insert("fetched_at".to_string(), chrono::Utc::now().to_rfc3339());
		metadata.insert("revision".to_string(), self.source.revision.clone());
		metadata.insert("remote".to_string(), self.source.remote.clone());
		metadata.insert("files_count".to_string(), content.len().to_string());

		Ok(CurrentGit {
			content,
			metadata,
			source: self.source.clone(),
			current_revision: self.source.revision.clone(),
		})
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
		assert_eq!(pattern.line_range, Some(LineRange::new(1, 10)?));
		assert_eq!(pattern.glob, None);

		// Test glob pattern
		let pattern = PathPattern::try_new("src/**/*.rs")?;
		assert_eq!(pattern.path, "src/**/*.rs");
		assert_eq!(pattern.line_range, None);
		assert_eq!(pattern.glob, Some("src/**/*.rs".to_string()));

		Ok(())
	}

	#[test]
	fn test_debug_line_range_parsing() -> Result<(), anyhow::Error> {
		// Debug test to see what's happening
		let test_str = "L1-L10";
		println!("Testing string: '{}'", test_str);

		if test_str.starts_with('L') {
			let stripped = &test_str[1..];
			println!("After stripping L: '{}'", stripped);

			let result = LineRange::try_from_string(stripped);
			println!("Result: {:?}", result);
		}

		Ok(())
	}

	#[test]
	fn test_git_source_creation() -> Result<(), anyhow::Error> {
		let source = GitSource::new(
			"https://github.com/user/repo.git",
			"main",
			"v1.0.0",
			vec!["src/lib.rs", "README.md"],
		)?;

		assert_eq!(source.remote, "https://github.com/user/repo.git");
		assert_eq!(source.default_branch, "main");
		assert_eq!(source.revision, "v1.0.0");
		assert_eq!(source.paths.len(), 2);

		Ok(())
	}

	#[test]
	fn test_git_match_creation() -> Result<(), anyhow::Error> {
		let git_match = GitMatch::new(
			"https://github.com/user/repo.git",
			"main",
			"v1.0.0",
			vec!["src/lib.rs"],
		)?;

		assert_eq!(git_match.source.remote, "https://github.com/user/repo.git");
		assert_eq!(git_match.source.revision, "v1.0.0");

		Ok(())
	}

	#[test]
	fn test_unified_diff_generation() -> Result<(), anyhow::Error> {
		let referenced = "Line 1\nOld Line 2\nLine 3";
		let current = "Line 1\nNew Line 2\nLine 3\nLine 4";
		let path = "test.rs";

		let diff = CurrentGit::generate_unified_diff(referenced, current, path);

		assert!(diff.contains("--- a/test.rs"));
		assert!(diff.contains("+++ b/test.rs"));
		assert!(diff.contains(" Line 1")); // unchanged
		assert!(diff.contains("-Old Line 2")); // removed
		assert!(diff.contains("+New Line 2")); // added
		assert!(diff.contains("+Line 4")); // new line

		Ok(())
	}
}
