use crate::GitSourceError;
use git2::{DiffFormat, DiffOptions};
use git2::{FetchOptions, RemoteCallbacks, Repository as Git2Repository};
use std::path::PathBuf;

/// Repository unifies all operations against a repository.
///
/// Unlike a [Git2Repository], [Repository] does not construct on a local repository.
/// Instead it describes an abstract reference to a repository and its intend file location.
#[derive(Debug, Clone, PartialEq)]
pub struct Repository {
	path: PathBuf,
	remote: String,
}

impl Repository {
	/// Create a new repository reference
	pub fn new(path: PathBuf, remote: String) -> Self {
		Self { path, remote }
	}
	/// Open the git repository at this path
	pub fn open_git_repo(&self) -> Result<Git2Repository, GitSourceError> {
		Git2Repository::open(&self.path).map_err(|e| GitSourceError::Git(e))
	}

	/// Update an existing repository (best-effort operation)
	pub fn update_from_remote(&mut self, remote_url: &str) -> Result<(), GitSourceError> {
		// Try to fetch latest changes - if it fails, that's okay too
		let _ = self.fetch_latest_changes(remote_url);
		Ok(())
	}

	/// Clone the repository from the remote URL
	fn clone_repository(&self) -> Result<(), GitSourceError> {
		// Ensure parent directory exists
		if let Some(parent) = self.path.parent() {
			std::fs::create_dir_all(parent).map_err(|e| {
				GitSourceError::InvalidRemote(format!("Failed to create parent directory: {}", e))
			})?;
		}

		match Git2Repository::clone(&self.remote, &self.path) {
			Ok(_repo) => Ok(()),
			Err(e) => {
				// Check if this is the "exists and is not an empty directory" error
				if e.code() == git2::ErrorCode::Exists
					&& e.message().contains("exists and is not an empty directory")
				{
					Ok(())
				} else {
					Err(GitSourceError::Git(e))
				}
			}
		}
	}

	/// Fetch latest changes for an existing repository
	fn fetch_latest_changes(&self, remote_url: &str) -> Result<(), GitSourceError> {
		let git_repo = self.open_git_repo()?;
		let mut remote = git_repo
			.find_remote("origin")
			.or_else(|_| git_repo.remote("origin", remote_url))
			.map_err(|e| GitSourceError::Git(e))?;

		let mut callbacks = RemoteCallbacks::new();
		callbacks.credentials(|_url, _username_from_url, _allowed_types| git2::Cred::default());

		let mut fetch_options = FetchOptions::new();
		fetch_options.remote_callbacks(callbacks);

		// Fetch all branches and tags to ensure we have the latest symbols
		remote
			.fetch(
				&["refs/heads/*:refs/remotes/origin/*", "refs/tags/*:refs/tags/*"],
				Some(&mut fetch_options),
				None,
			)
			.map_err(|e| GitSourceError::Git(e))?;

		Ok(())
	}

	/// Generate a simple directory name for a repository
	pub fn generate_repo_dir_name(remote_url: &str) -> String {
		// Extract the repo name from the URL
		// e.g., "https://github.com/ramate-io/cite.git" -> "cite"
		// e.g., "https://github.com/user/repo-name.git" -> "repo-name"
		if let Some(last_part) = remote_url.split('/').last() {
			if last_part.ends_with(".git") {
				return last_part[..last_part.len() - 4].to_string();
			}
			return last_part.to_string();
		}

		// Fallback: use a sanitized version of the URL
		remote_url.replace([':', '/', '.'], "_")
	}

	/// Get the target directory for git repositories
	pub fn get_target_dir(parent_dir: Option<PathBuf>) -> Result<PathBuf, GitSourceError> {
		let base_dir = if let Some(parent_dir) = parent_dir {
			parent_dir
		} else {
			// Use CARGO_TARGET_DIR if set, otherwise default to target/cite-git
			std::env::var("CARGO_TARGET_DIR")
				.map(PathBuf::from)
				.unwrap_or_else(|_| PathBuf::from("target"))
				.join("cite-git")
		};

		std::fs::create_dir_all(&base_dir).map_err(|e| {
			GitSourceError::InvalidRemote(format!("Failed to create target directory: {}", e))
		})?;

		Ok(base_dir)
	}

	/// Get the repository path
	pub fn path(&self) -> &PathBuf {
		&self.path
	}

	/// Get the remote URL
	pub fn remote(&self) -> &str {
		&self.remote
	}

	/// Check if a revision exists in the repository
	pub fn revision_exists(&self, revision: &str) -> Result<bool, GitSourceError> {
		let git_repo = self.open_git_repo()?;

		// First try the revision as-is
		if git_repo.revparse_single(revision).is_ok() {
			return Ok(true);
		}

		// If it's a branch name, try common branch reference patterns
		if !revision.starts_with("refs/") && !revision.chars().all(|c| c.is_ascii_hexdigit()) {
			// Try origin/branch pattern
			if git_repo.revparse_single(&format!("origin/{}", revision)).is_ok() {
				return Ok(true);
			}

			// Try refs/heads/branch pattern
			if git_repo.revparse_single(&format!("refs/heads/{}", revision)).is_ok() {
				return Ok(true);
			}

			// Try refs/remotes/origin/branch pattern
			if git_repo.revparse_single(&format!("refs/remotes/origin/{}", revision)).is_ok() {
				return Ok(true);
			}
		}

		Ok(false)
	}

	/// Convert a revision string to a proper refspec format for fetching
	pub fn convert_to_refspec(&self, revision: &str) -> String {
		// If it looks like a commit hash (40 characters, hex), fetch it directly
		if revision.len() == 40 && revision.chars().all(|c| c.is_ascii_hexdigit()) {
			return revision.to_string();
		}

		// If it looks like a short commit hash (7-39 characters, hex), fetch it directly
		if revision.len() >= 7
			&& revision.len() <= 39
			&& revision.chars().all(|c| c.is_ascii_hexdigit())
		{
			return revision.to_string();
		}

		// For branch names, use the proper refspec format
		// This handles both local branch names and remote branch names
		if !revision.starts_with("refs/") {
			// Try origin/branch first, then refs/heads/branch
			format!("refs/heads/{}:refs/remotes/origin/{}", revision, revision)
		} else {
			// Already a refspec, use as-is
			revision.to_string()
		}
	}

	/// Clone repository and ensure trees for revisions (mutable operation)
	pub fn clone_and_ensure_revisions(
		&mut self,
		revisions: &[String],
	) -> Result<(), GitSourceError> {
		// If repository doesn't exist, clone it first
		if !self.path.exists() {
			self.clone_repository()?;
		}

		let git_repo = self.open_git_repo()?;
		let mut remote = git_repo.find_remote("origin").map_err(|e| GitSourceError::Git(e))?;

		let mut callbacks = RemoteCallbacks::new();
		callbacks.credentials(|_url, _username_from_url, _allowed_types| git2::Cred::default());

		let mut fetch_options = FetchOptions::new();
		fetch_options.remote_callbacks(callbacks);

		// Collect revisions that need fetching
		let mut revisions_to_fetch = Vec::new();
		for revision in revisions {
			if !self.revision_exists(revision)? {
				revisions_to_fetch.push(self.convert_to_refspec(revision));
			}
		}

		// Only fetch if we have revisions that don't exist locally
		if !revisions_to_fetch.is_empty() {
			// Fetch the specific revisions we need
			remote
				.fetch(&revisions_to_fetch, Some(&mut fetch_options), None)
				.map_err(|e| GitSourceError::Git(e))?;
		}

		// Validate that we can resolve each revision (this will lazily fetch content as needed)
		for revision in revisions {
			if self.revision_exists(revision)? {
				// Try to resolve the revision - this will fetch content lazily if needed
				if let Ok(obj) = git_repo.revparse_single(revision) {
					match obj.kind() {
						Some(git2::ObjectType::Commit) => {
							// For commits, just verify we can access the tree (lazy fetch)
							if let Ok(commit) = obj.peel_to_commit() {
								let _tree = commit.tree().map_err(|e| GitSourceError::Git(e))?;
							}
						}
						Some(git2::ObjectType::Tag) => {
							// For tags, peel to commit and verify tree access
							if let Ok(tag) = obj.peel_to_tag() {
								if let Ok(target) = tag.target() {
									if let Ok(commit) = target.peel_to_commit() {
										let _tree =
											commit.tree().map_err(|e| GitSourceError::Git(e))?;
									}
								}
							}
						}
						Some(git2::ObjectType::Tree) => {
							// For trees, verify we can access it
							let _tree = obj.peel_to_tree().map_err(|e| GitSourceError::Git(e))?;
						}
						_ => {
							// Other object types, just verify resolution
							let _ = git_repo.revparse_single(revision);
						}
					}
				}
			}
		}

		Ok(())
	}

	/// Get content diff buffer between two revisions, filtered by path pattern
	pub fn get_content_diff_buffer(
		&self,
		referenced: &str,
		current: &str,
		path_pattern: &crate::PathPattern,
	) -> Result<String, GitSourceError> {
		let repo = self.open_git_repo()?;

		let obj = repo.revparse_single(referenced).map_err(|e| GitSourceError::Git(e.into()))?;

		let comparison_tree = match obj.kind() {
			Some(git2::ObjectType::Commit) => {
				let commit = obj.peel_to_commit().map_err(|e| GitSourceError::Git(e.into()))?;
				commit.tree().map_err(|e| GitSourceError::Git(e.into()))?
			}
			Some(git2::ObjectType::Tag) => {
				let tag = obj.peel_to_tag().map_err(|e| GitSourceError::Git(e.into()))?;
				let target = tag.target().map_err(|e| GitSourceError::Git(e.into()))?;
				let commit = target.peel_to_commit().map_err(|e| GitSourceError::Git(e.into()))?;
				commit.tree().map_err(|e| GitSourceError::Git(e.into()))?
			}
			Some(git2::ObjectType::Tree) => {
				obj.peel_to_tree().map_err(|e| GitSourceError::Git(e.into()))?
			}
			_ => {
				return Err(GitSourceError::InvalidRevision(
					format!("Invalid revision type: {}", referenced).into(),
				))
			}
		};

		// Get the current revision's tree for comparison
		let current_obj =
			repo.revparse_single(current).map_err(|e| GitSourceError::Git(e.into()))?;

		let current_tree = match current_obj.kind() {
			Some(git2::ObjectType::Commit) => {
				let commit =
					current_obj.peel_to_commit().map_err(|e| GitSourceError::Git(e.into()))?;
				commit.tree().map_err(|e| GitSourceError::Git(e.into()))?
			}
			Some(git2::ObjectType::Tag) => {
				let tag = current_obj.peel_to_tag().map_err(|e| GitSourceError::Git(e.into()))?;
				let target = tag.target().map_err(|e| GitSourceError::Git(e.into()))?;
				let commit = target.peel_to_commit().map_err(|e| GitSourceError::Git(e.into()))?;
				commit.tree().map_err(|e| GitSourceError::Git(e.into()))?
			}
			Some(git2::ObjectType::Tree) => {
				current_obj.peel_to_tree().map_err(|e| GitSourceError::Git(e.into()))?
			}
			_ => {
				return Err(GitSourceError::InvalidRevision(
					format!("Invalid current revision type: {}", current).into(),
				))
			}
		};

		// Compare the two trees: referenced_revision vs current_revision
		let mut opts = DiffOptions::new();

		// Handle different path patterns for diff options
		if let Some(ref _glob_pattern) = path_pattern.glob {
			// For glob patterns, we need to handle the filtering in the diff callback
			// Don't set pathspec for glob patterns as git2 doesn't support glob in pathspec
		} else if path_pattern.path.ends_with('/') {
			// For directory paths, we need to handle this differently
			// Don't set pathspec for directories as it won't work properly
		} else {
			// For single files, we can use pathspec
			opts.pathspec(&path_pattern.path);
		}

		let diff = repo
			.diff_tree_to_tree(Some(&comparison_tree), Some(&current_tree), Some(&mut opts))
			.map_err(|e| GitSourceError::Git(e.into()))?;

		// Capture the diff output and check for intersections
		let mut buffer = String::new();
		let mut has_changes = false;

		diff.print(DiffFormat::Patch, |delta, _hunk, line| {
			// Check if this delta affects a file that matches our pattern
			let file_path = delta.new_file().path().or_else(|| delta.old_file().path());

			if let Some(path) = file_path {
				// Enhanced path matching for directories and glob patterns
				let path_matches = if path_pattern.glob.is_some() {
					// For glob patterns, use the existing matches method
					path_pattern.matches(path)
				} else if path_pattern.path.ends_with('/') {
					// For directory paths (with or without trailing slash), check if the file is within the directory
					let dir_path = path_pattern.path.trim_end_matches('/');
					path.to_string_lossy().starts_with(dir_path)
						&& (path.to_string_lossy() == dir_path
							|| path.to_string_lossy().starts_with(&format!("{}/", dir_path)))
				} else {
					// For single files, use exact match
					path_pattern.matches(path)
				};

				if path_matches {
					// Check if this line is within our line range
					let should_include = if let Some(ref line_range) = path_pattern.line_range {
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
		.map_err(|e| GitSourceError::Git(e))?;

		Ok(buffer)
	}
}
