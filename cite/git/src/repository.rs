pub mod builder;
pub mod lock;
pub mod repository;

pub use repository::Repository;

pub use builder::RepositoryBuilder;

use crate::GitSourceError;
use git2::{FetchOptions, RemoteCallbacks, Repository};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Manages operations on a cached git repository
#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryManager {
	repo_path: PathBuf,
}

impl RepositoryManager {
	/// Create a new repository manager for the given repository path
	pub fn new(repo_path: PathBuf) -> Self {
		Self { repo_path }
	}

	/// Get the repository path
	pub fn path(&self) -> &PathBuf {
		&self.repo_path
	}

	/// Check if a revision exists in the repository
	pub fn revision_exists(&self, revision: &str) -> bool {
		let repo = match Repository::open(&self.repo_path) {
			Ok(repo) => repo,
			Err(_) => return false,
		};

		// First try the revision as-is
		if repo.revparse_single(revision).is_ok() {
			return true;
		}

		// If it's a branch name, try common branch reference patterns
		if !revision.starts_with("refs/") && !revision.chars().all(|c| c.is_ascii_hexdigit()) {
			// Try origin/branch pattern
			if repo.revparse_single(&format!("origin/{}", revision)).is_ok() {
				return true;
			}

			// Try refs/heads/branch pattern
			if repo.revparse_single(&format!("refs/heads/{}", revision)).is_ok() {
				return true;
			}

			// Try refs/remotes/origin/branch pattern
			if repo.revparse_single(&format!("refs/remotes/origin/{}", revision)).is_ok() {
				return true;
			}
		}

		false
	}

	/// Get the repository at the managed path
	pub fn get_repository(&self) -> Result<Repository, GitSourceError> {
		Repository::open(&self.repo_path).map_err(|e| GitSourceError::Git(e))
	}

	/// Retry git operations that fail due to lock conflicts
	fn with_retry<F, T>(&self, operation: F) -> Result<T, GitSourceError>
	where
		F: Fn() -> Result<T, GitSourceError>,
	{
		let max_retries = 10;
		let base_delay = std::time::Duration::from_millis(50);

		for attempt in 0..max_retries {
			match operation() {
				Ok(result) => return Ok(result),
				Err(e) => {
					// Check if this is a lock-related error
					if self.is_lock_error(&e) {
						if attempt < max_retries - 1 {
							// Exponential backoff: 50ms, 100ms, 200ms, 400ms
							let delay = base_delay * (1 << attempt);
							std::thread::sleep(delay);
							continue;
						}
					}
					return Err(e);
				}
			}
		}

		// This should never be reached due to the match above, but just in case
		operation()
	}

	/// Check if an error is related to git lock conflicts
	fn is_lock_error(&self, error: &GitSourceError) -> bool {
		match error {
			GitSourceError::Git(git_error) => {
				let message = git_error.message();
				message.contains("lock")
					|| message.contains("File exists")
					|| message.contains("failed to create locked file")
			}
			_ => false,
		}
	}

	/// Fetch specific revisions that are needed
	pub fn fetch_specific_revisions(&self, revisions: &[&str]) -> Result<(), GitSourceError> {
		self.with_retry(|| {
			let repo = Repository::open(&self.repo_path).map_err(|e| GitSourceError::Git(e))?;
			let mut remote = repo.find_remote("origin").map_err(|e| GitSourceError::Git(e))?;

			let mut callbacks = RemoteCallbacks::new();
			callbacks.credentials(|_url, _username_from_url, _allowed_types| git2::Cred::default());

			let mut fetch_options = FetchOptions::new();
			fetch_options.remote_callbacks(callbacks);

			// Collect revisions that need fetching
			let mut revisions_to_fetch = Vec::new();
			for revision in revisions {
				if !self.revision_exists(revision) {
					revisions_to_fetch.push(self.convert_to_refspec(revision));
				}
			}

			// Only fetch if we have revisions that don't exist locally
			if !revisions_to_fetch.is_empty() {
				// First try to fetch just the specific revisions we need
				let fetch_result =
					remote.fetch(&revisions_to_fetch, Some(&mut fetch_options), None);

				// If specific fetch fails, try fetching common branches that might contain our revisions
				if fetch_result.is_err() {
					// Try fetching main/master branches which are likely to contain most commits
					let common_branches = [
						"refs/heads/main:refs/remotes/origin/main",
						"refs/heads/master:refs/remotes/origin/master",
					];
					let _ = remote.fetch(&common_branches, Some(&mut fetch_options), None);

					// Try fetching the specific revisions again
					let _ = remote.fetch(&revisions_to_fetch, Some(&mut fetch_options), None);
				}
			}

			// Validate that we can resolve each revision (this will lazily fetch content as needed)
			for revision in revisions {
				if self.revision_exists(revision) {
					// Try to resolve the revision - this will fetch content lazily if needed
					if let Ok(obj) = repo.revparse_single(revision) {
						match obj.kind() {
							Some(git2::ObjectType::Commit) => {
								// For commits, just verify we can access the tree (lazy fetch)
								if let Ok(commit) = obj.peel_to_commit() {
									let _tree =
										commit.tree().map_err(|e| GitSourceError::Git(e))?;
								}
							}
							Some(git2::ObjectType::Tag) => {
								// For tags, peel to commit and verify tree access
								if let Ok(tag) = obj.peel_to_tag() {
									if let Ok(target) = tag.target() {
										if let Ok(commit) = target.peel_to_commit() {
											let _tree = commit
												.tree()
												.map_err(|e| GitSourceError::Git(e))?;
										}
									}
								}
							}
							Some(git2::ObjectType::Tree) => {
								// For trees, verify we can access it
								let _tree =
									obj.peel_to_tree().map_err(|e| GitSourceError::Git(e))?;
							}
							_ => {
								// Other object types, just verify resolution
								let _ = repo.revparse_single(revision);
							}
						}
					}
				}
			}

			Ok(())
		})
	}

	/// Convert a revision string to a proper refspec format for fetching
	fn convert_to_refspec(&self, revision: &str) -> String {
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
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_repo_dir_name() {
		assert_eq!(
			RepositoryBuilder::generate_repo_dir_name("https://github.com/ramate-io/cite.git"),
			"cite"
		);
		assert_eq!(
			RepositoryBuilder::generate_repo_dir_name("https://github.com/user/repo-name.git"),
			"repo-name"
		);
		assert_eq!(
			RepositoryBuilder::generate_repo_dir_name("https://gitlab.com/group/project"),
			"project"
		);
	}

	#[test]
	fn test_get_target_dir() {
		let builder = RepositoryBuilder::new("https://github.com/ramate-io/cite.git".to_string());
		let target_dir = builder.get_target_dir().unwrap();
		assert!(target_dir.exists());
		assert!(target_dir.is_dir());
	}

	#[test]
	fn test_repository_builder_new() {
		let builder = RepositoryBuilder::new("https://github.com/ramate-io/cite".to_string());
		assert_eq!(builder.remote_url, "https://github.com/ramate-io/cite");
	}

	#[test]
	fn test_repository_manager_new() {
		let manager = RepositoryManager::new(PathBuf::from("/tmp/test"));
		assert_eq!(manager.path(), &PathBuf::from("/tmp/test"));
	}

	#[test]
	fn test_repository_fetch_new_repo() {
		// Test fetching a new repository
		let builder = RepositoryBuilder::new("https://github.com/ramate-io/cite.git".to_string());
		let result = builder.fetch();

		// This should succeed and create the repository
		assert!(result.is_ok());

		let manager = result.unwrap();
		assert!(manager.repo_path.exists());
		assert!(manager.repo_path.join(".git").exists());

		// Test that we can get the repository
		let repo = manager.get_repository().unwrap();
		assert!(!repo.is_bare());

		// Test that the revision exists
		assert!(manager.revision_exists("main"));
	}

	#[test]
	fn test_repository_fetch_existing_repo() {
		// Test fetching an existing repository (should update it)
		let builder = RepositoryBuilder::new("https://github.com/ramate-io/cite.git".to_string());
		let result = builder.fetch();

		// This should succeed
		assert!(result.is_ok());

		let manager = result.unwrap();
		assert!(manager.repo_path.exists());

		// Test that we can still get the repository after update
		let repo = manager.get_repository().unwrap();
		assert!(!repo.is_bare());
	}

	#[test]
	fn test_revision_exists() {
		let builder = RepositoryBuilder::new("https://github.com/ramate-io/cite.git".to_string());
		let manager = builder.fetch().unwrap();

		// Test that main branch exists
		assert!(manager.revision_exists("main"));

		// Test that a specific commit exists (using a known commit from the repo)
		assert!(manager.revision_exists("7a6e85985fbfb8f2035a66bccb047ea46d419d78"));

		// Test that a non-existent revision returns false
		assert!(!manager.revision_exists("nonexistent-commit"));
	}

	#[test]
	fn test_custom_parent_dir() {
		// Test using a custom parent directory
		let temp_dir = std::env::temp_dir().join("cite-git-test");
		let builder = RepositoryBuilder::with_parent_dir(
			"https://github.com/ramate-io/cite.git".to_string(),
			temp_dir.clone(),
		);

		let target_dir = builder.get_target_dir().unwrap();
		assert_eq!(target_dir, temp_dir);

		// Test fetching with custom directory
		let result = builder.fetch();
		assert!(result.is_ok());

		let manager = result.unwrap();
		assert!(manager.repo_path.starts_with(&temp_dir));
		assert!(manager.repo_path.exists());

		// Clean up
		let _ = std::fs::remove_dir_all(&temp_dir);
	}

	#[test]
	fn test_fetch_specific_revisions() {
		let temp_dir = tempfile::tempdir().unwrap();
		let builder = RepositoryBuilder::with_parent_dir(
			"https://github.com/ramate-io/cite".to_string(),
			temp_dir.path().to_path_buf(),
		);
		let manager = builder.fetch().unwrap();

		// Test fetching a specific commit hash
		let result =
			manager.fetch_specific_revisions(&["94dab273cf6c2abe8742d6d459ad45c96ca9b694"]);
		assert!(result.is_ok());

		// Verify the revision now exists
		assert!(manager.revision_exists("94dab273cf6c2abe8742d6d459ad45c96ca9b694"));

		// Test fetching multiple revisions
		let result =
			manager.fetch_specific_revisions(&["main", "2bcceb14934dbe0803ddb70bc8952a0c33f931e2"]);
		assert!(result.is_ok());

		// Verify both revisions exist
		assert!(manager.revision_exists("main"));
		assert!(manager.revision_exists("2bcceb14934dbe0803ddb70bc8952a0c33f931e2"));
	}

	#[test]
	fn test_convert_to_refspec() {
		let temp_dir = tempfile::tempdir().unwrap();
		let builder = RepositoryBuilder::with_parent_dir(
			"https://github.com/ramate-io/cite".to_string(),
			temp_dir.path().to_path_buf(),
		);
		let manager = builder.fetch().unwrap();

		// Test commit hash (40 characters)
		let refspec = manager.convert_to_refspec("94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(refspec, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");

		// Test short commit hash (7 characters)
		let refspec = manager.convert_to_refspec("94dab27");
		assert_eq!(refspec, "94dab27");

		// Test branch name
		let refspec = manager.convert_to_refspec("main");
		assert_eq!(refspec, "refs/heads/main:refs/remotes/origin/main");

		// Test branch name with special characters
		let refspec = manager.convert_to_refspec("feature/new-feature");
		assert_eq!(
			refspec,
			"refs/heads/feature/new-feature:refs/remotes/origin/feature/new-feature"
		);

		// Test already formatted refspec
		let refspec = manager.convert_to_refspec("refs/heads/main");
		assert_eq!(refspec, "refs/heads/main");
	}

	#[test]
	fn test_revision_exists_branch_patterns() {
		let temp_dir = tempfile::tempdir().unwrap();
		let builder = RepositoryBuilder::with_parent_dir(
			"https://github.com/ramate-io/cite".to_string(),
			temp_dir.path().to_path_buf(),
		);
		let manager = builder.fetch().unwrap();

		// Test that main branch exists (should work with the improved detection)
		assert!(manager.revision_exists("main"));

		// Test commit hash exists
		assert!(manager.revision_exists("94dab273cf6c2abe8742d6d459ad45c96ca9b694"));

		// Test that non-existent revision returns false
		assert!(!manager.revision_exists("nonexistent-branch"));
		assert!(!manager.revision_exists("nonexistent-commit-hash"));
	}

	#[test]
	fn test_ramate_oac_repository() {
		// Test with the ramate-io/oac repository to debug empty repo issues
		let temp_dir = tempfile::tempdir().unwrap();
		let builder = RepositoryBuilder::with_parent_dir(
			"https://github.com/ramate-io/oac".to_string(),
			temp_dir.path().to_path_buf(),
		);

		let manager = builder.fetch().unwrap();
		let repo_path = manager.path();

		// Verify the repository directory exists and has .git
		assert!(repo_path.exists());
		assert!(repo_path.join(".git").exists());

		// Test that we can open the repository
		let repo = manager.get_repository().unwrap();
		assert!(!repo.is_bare());

		// Test common branch names
		let common_branches = ["main", "master", "develop", "dev"];
		let mut found_branches = Vec::new();

		for branch in &common_branches {
			if manager.revision_exists(branch) {
				found_branches.push(*branch);
			}
		}

		// At least one branch should exist
		assert!(!found_branches.is_empty(), "No common branches found in ramate-io/oac repository");

		// Test fetching the found branches
		if !found_branches.is_empty() {
			let result = manager.fetch_specific_revisions(&found_branches);
			assert!(result.is_ok());
		}

		// Test that we can list references
		let references = repo.references().unwrap();
		let ref_count = references.count();
		assert!(ref_count > 0, "Repository appears to have no references");

		// Test that HEAD exists
		assert!(repo.head().is_ok(), "Repository HEAD not found");
	}

	#[test]
	fn test_repository_diagnostics() {
		// Test diagnostic functions to inspect repository state
		let temp_dir = tempfile::tempdir().unwrap();
		let builder = RepositoryBuilder::with_parent_dir(
			"https://github.com/ramate-io/oac".to_string(),
			temp_dir.path().to_path_buf(),
		);

		let manager = builder.fetch().unwrap();
		let repo = manager.get_repository().unwrap();

		// Test repository diagnostics
		let head = repo.head().unwrap();
		assert!(head.name().is_some(), "HEAD should have a name");

		// List all references
		let references = repo.references().unwrap();
		let ref_count = references.count();
		assert!(ref_count > 0, "Repository should have references");

		// Test branch listing
		let branches = repo.branches(None).unwrap();
		let branch_count = branches.count();
		assert!(branch_count > 0, "Repository should have branches");

		// Test remote listing
		let remotes = repo.remotes().unwrap();
		assert!(!remotes.is_empty(), "Repository should have remotes");

		// Verify repository is not empty
		assert!(!repo.is_empty().unwrap(), "Repository should not be empty");
	}
}
