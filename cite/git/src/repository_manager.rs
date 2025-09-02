use crate::GitSourceError;
use git2::{FetchOptions, RemoteCallbacks, Repository};
use std::path::{Path, PathBuf};

/// Builder for fetching and preparing git repositories
#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryBuilder {
	remote_url: String,
	parent_dir: Option<PathBuf>,
}

impl Default for RepositoryBuilder {
	fn default() -> Self {
		Self { remote_url: String::new(), parent_dir: None }
	}
}

impl RepositoryBuilder {
	/// Create a new repository builder for the given remote URL
	pub fn new(remote_url: String) -> Self {
		Self { remote_url, parent_dir: None }
	}

	/// Create a new repository builder with a custom parent directory
	pub fn with_parent_dir(remote_url: String, parent_dir: PathBuf) -> Self {
		Self { remote_url, parent_dir: Some(parent_dir) }
	}

	/// Get the target directory for git repositories
	pub fn get_target_dir(&self) -> Result<PathBuf, GitSourceError> {
		let base_dir = if let Some(ref parent_dir) = self.parent_dir {
			parent_dir.clone()
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

	/// Fetch the repository and return a RepositoryManager
	pub fn fetch(self) -> Result<RepositoryManager, GitSourceError> {
		let target_dir = self.get_target_dir()?;
		let repo_dir_name = Self::generate_repo_dir_name(&self.remote_url);
		let repo_path = target_dir.join(repo_dir_name);

		// If the repository already exists, check if we need to update it
		if repo_path.exists() {
			// Try to update the repository to get latest changes
			// This is a best-effort operation - if it fails, we'll still use the existing repo
			let _ = Self::update_existing_repository(&repo_path, &self.remote_url);
		} else {
			// Clone the repository
			let mut callbacks = RemoteCallbacks::new();
			callbacks.credentials(|_url, _username_from_url, _allowed_types| git2::Cred::default());

			let mut fetch_options = FetchOptions::new();
			fetch_options.remote_callbacks(callbacks);

			let _repo = Repository::clone(&self.remote_url, &repo_path)
				.map_err(|e| GitSourceError::Git(e))?;
		}

		Ok(RepositoryManager::new(repo_path))
	}

	/// Update an existing repository
	fn update_existing_repository(
		repo_path: &Path,
		remote_url: &str,
	) -> Result<(), GitSourceError> {
		let repo = Repository::open(repo_path).map_err(|e| GitSourceError::Git(e))?;
		Self::fetch_latest_changes(&repo, remote_url)
	}

	/// Fetch latest changes for an existing repository
	fn fetch_latest_changes(repo: &Repository, remote_url: &str) -> Result<(), GitSourceError> {
		let mut remote = repo
			.find_remote("origin")
			.or_else(|_| repo.remote("origin", remote_url))
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
}

/// Manages operations on a cached git repository
#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryManager {
	repo_path: PathBuf,
}

impl Default for RepositoryManager {
	fn default() -> Self {
		Self { repo_path: PathBuf::new() }
	}
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

		let result = repo.revparse_single(revision).is_ok();
		result
	}

	/// Get the repository at the managed path
	pub fn get_repository(&self) -> Result<Repository, GitSourceError> {
		Repository::open(&self.repo_path).map_err(|e| GitSourceError::Git(e))
	}

	/// Fetch specific revisions that are needed
	pub fn fetch_specific_revisions(&self, revisions: &[&str]) -> Result<(), GitSourceError> {
		let repo = Repository::open(&self.repo_path).map_err(|e| GitSourceError::Git(e))?;
		let mut remote = repo.find_remote("origin").map_err(|e| GitSourceError::Git(e))?;

		let mut callbacks = RemoteCallbacks::new();
		callbacks.credentials(|_url, _username_from_url, _allowed_types| git2::Cred::default());

		let mut fetch_options = FetchOptions::new();
		fetch_options.remote_callbacks(callbacks);

		// For each revision, try to fetch it if it doesn't exist locally
		for revision in revisions {
			if !self.revision_exists(revision) {
				// Try to fetch this specific commit
				// Note: This is a best-effort approach - some commits might not be fetchable
				// if they're not reachable from any ref
				let _ = remote.fetch(&[revision], Some(&mut fetch_options), None);
			}
		}

		Ok(())
	}
}

// Legacy functions for backward compatibility
pub fn fetch_repository(remote_url: &str) -> Result<PathBuf, GitSourceError> {
	let builder = RepositoryBuilder::new(remote_url.to_string());
	builder.fetch().map(|manager| manager.path().clone())
}

pub fn revision_exists(repo_path: &Path, revision: &str) -> bool {
	let manager = RepositoryManager::new(repo_path.to_path_buf());
	manager.revision_exists(revision)
}

pub fn get_repository(repo_path: &Path) -> Result<Repository, GitSourceError> {
	let manager = RepositoryManager::new(repo_path.to_path_buf());
	manager.get_repository()
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
}
