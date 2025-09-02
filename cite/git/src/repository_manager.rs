use crate::GitSourceError;
use git2::{FetchOptions, RemoteCallbacks, Repository};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Builder for fetching and preparing git repositories
#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryBuilder {
	remote_url: String,
}

impl Default for RepositoryBuilder {
	fn default() -> Self {
		Self { remote_url: String::new() }
	}
}

impl RepositoryBuilder {
	/// Create a new repository builder for the given remote URL
	pub fn new(remote_url: String) -> Self {
		Self { remote_url }
	}

	/// Get the target directory for git repositories
	fn get_target_dir() -> Result<PathBuf, GitSourceError> {
		// Use CARGO_TARGET_DIR if set, otherwise default to target/cite-git
		let target_dir = std::env::var("CARGO_TARGET_DIR")
			.map(PathBuf::from)
			.unwrap_or_else(|_| PathBuf::from("target"))
			.join("cite-git");

		std::fs::create_dir_all(&target_dir).map_err(|e| {
			GitSourceError::InvalidRemote(format!("Failed to create target directory: {}", e))
		})?;

		Ok(target_dir)
	}

	/// Generate a unique directory name for a repository
	fn generate_repo_dir_name(remote_url: &str) -> String {
		let mut hasher = DefaultHasher::new();
		remote_url.hash(&mut hasher);
		let hash = hasher.finish();

		format!("repo_{:x}", hash)
	}

	/// Fetch the repository and return a RepositoryManager
	pub fn fetch(self) -> Result<RepositoryManager, GitSourceError> {
		let target_dir = Self::get_target_dir()?;
		let repo_dir_name = Self::generate_repo_dir_name(&self.remote_url);
		let repo_path = target_dir.join(repo_dir_name);

		// If the repository already exists, just update it
		if repo_path.exists() {
			Self::update_existing_repository(&repo_path, &self.remote_url)?;
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

		remote
			.fetch(&["refs/heads/*:refs/remotes/origin/*"], Some(&mut fetch_options), None)
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
		let dir1 = RepositoryBuilder::generate_repo_dir_name("https://github.com/ramate-io/cite");
		let dir2 = RepositoryBuilder::generate_repo_dir_name("https://github.com/ramate-io/cite");
		let dir3 = RepositoryBuilder::generate_repo_dir_name("https://github.com/other/repo");

		assert_eq!(dir1, dir2);
		assert_ne!(dir1, dir3);
		assert!(dir1.starts_with("repo_"));
	}

	#[test]
	fn test_get_target_dir() {
		let target_dir = RepositoryBuilder::get_target_dir().unwrap();
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
}
