pub mod analyzer;
pub mod builder;
pub mod lock;
pub mod repository;

pub use repository::Repository;

pub use analyzer::RepositoryManager;
pub use builder::RepositoryBuilder;

use crate::GitSourceError;
use std::path::PathBuf;

/// Manages operations on a cached git repository with advisory locking
#[derive(Debug, Clone)]
pub struct RepositoryManager {
	pub repository: Repository,
	pub lock_file: lock::LockFile,
}

impl RepositoryManager {
	/// Create a new repository manager for the given repository path
	pub fn new(repo_path: PathBuf) -> Result<Self, GitSourceError> {
		let repository = Repository::new(repo_path.clone())?;
		let lock_file = lock::LockFile { path: repo_path.join(".cite-lock"), is_writing: false };
		Ok(Self { repository, lock_file })
	}

	/// Create a new repository manager from an existing repository
	pub fn from_repository(repository: Repository) -> Result<Self, GitSourceError> {
		let lock_file =
			lock::LockFile { path: repository.path().join(".cite-lock"), is_writing: false };
		Ok(Self { repository, lock_file })
	}

	/// Get the repository path
	pub fn path(&self) -> &PathBuf {
		self.repository.path()
	}

	/// Check if a revision exists in the repository (read-only operation)
	pub fn revision_exists(&self, revision: &str) -> Result<bool, GitSourceError> {
		// For now, just use the repository directly without locking
		// TODO: Implement proper advisory locking
		Ok(self.repository.revision_exists(revision))
	}

	/// Get read access to the repository
	pub fn read(&self) -> Result<lock::ReadGuard, GitSourceError> {
		let lock = lock::Lock::new(self.repository.clone(), self.lock_file.clone());
		lock.read()
	}

	/// Get write access to the repository
	pub fn write(&mut self) -> Result<lock::WriteGuard, GitSourceError> {
		let lock = lock::Lock::new(self.repository.clone(), self.lock_file.clone());
		lock.write()
	}

	/// Fetch specific revisions that are needed (mutable operation)
	pub fn fetch_specific_revisions(&mut self, revisions: &[&str]) -> Result<(), GitSourceError> {
		let mut write_guard = self.write()?;
		write_guard.fetch_specific_revisions(revisions)
	}

	/// Get the underlying git2 repository (for compatibility with existing code)
	pub fn get_repository(&self) -> Result<git2::Repository, GitSourceError> {
		git2::Repository::open(self.repository.path()).map_err(|e| GitSourceError::Git(e))
	}

	/// Convert a revision string to a proper refspec format for fetching
	pub fn convert_to_refspec(&self, revision: &str) -> String {
		self.repository.convert_to_refspec(revision)
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
		// Test that the builder was created successfully
		assert!(builder.get_target_dir().is_ok());
	}

	#[test]
	fn test_repository_manager_new() {
		// This test would require a real git repository
		// For now, just test that the method signature is correct
		let result = RepositoryManager::new(PathBuf::from("/tmp/nonexistent"));
		assert!(result.is_err()); // Should fail for nonexistent repo
	}

	#[test]
	fn test_repository_fetch_new_repo() {
		// Test fetching a new repository
		let builder = RepositoryBuilder::new("https://github.com/ramate-io/cite.git".to_string());
		let result = builder.fetch();

		// This should succeed and create the repository
		assert!(result.is_ok());

		let manager = result.unwrap();
		assert!(manager.path().exists());
		assert!(manager.path().join(".git").exists());

		// Test that we can get the repository
		let repo = manager.get_repository().unwrap();
		assert!(!repo.is_bare());

		// Test that the revision exists
		assert!(manager.revision_exists("main").unwrap());
	}

	#[test]
	fn test_repository_fetch_existing_repo() {
		// Test fetching an existing repository (should update it)
		let builder = RepositoryBuilder::new("https://github.com/ramate-io/cite.git".to_string());
		let result = builder.fetch();

		// This should succeed
		assert!(result.is_ok());

		let manager = result.unwrap();
		assert!(manager.path().exists());

		// Test that we can still get the repository after update
		let repo = manager.get_repository().unwrap();
		assert!(!repo.is_bare());
	}

	#[test]
	fn test_revision_exists() {
		let builder = RepositoryBuilder::new("https://github.com/ramate-io/cite.git".to_string());
		let manager = builder.fetch().unwrap();

		// Test that main branch exists
		assert!(manager.revision_exists("main").unwrap());

		// Test that a specific commit exists (using a known commit from the repo)
		assert!(manager.revision_exists("7a6e85985fbfb8f2035a66bccb047ea46d419d78").unwrap());

		// Test that a non-existent revision returns false
		assert!(!manager.revision_exists("nonexistent-commit").unwrap());
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
		assert!(manager.path().starts_with(&temp_dir));
		assert!(manager.path().exists());

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
		let mut manager = builder.fetch().unwrap();

		// Test fetching a specific commit hash
		let result =
			manager.fetch_specific_revisions(&["94dab273cf6c2abe8742d6d459ad45c96ca9b694"]);
		assert!(result.is_ok());

		// Verify the revision now exists
		assert!(manager.revision_exists("94dab273cf6c2abe8742d6d459ad45c96ca9b694").unwrap());

		// Test fetching multiple revisions
		let result =
			manager.fetch_specific_revisions(&["main", "2bcceb14934dbe0803ddb70bc8952a0c33f931e2"]);
		assert!(result.is_ok());

		// Verify both revisions exist
		assert!(manager.revision_exists("main").unwrap());
		assert!(manager.revision_exists("2bcceb14934dbe0803ddb70bc8952a0c33f931e2").unwrap());
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
		assert!(manager.revision_exists("main").unwrap());

		// Test commit hash exists
		assert!(manager.revision_exists("94dab273cf6c2abe8742d6d459ad45c96ca9b694").unwrap());

		// Test that non-existent revision returns false
		assert!(!manager.revision_exists("nonexistent-branch").unwrap());
		assert!(!manager.revision_exists("nonexistent-commit-hash").unwrap());
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
			if manager.revision_exists(branch).unwrap() {
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
