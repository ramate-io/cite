use crate::GitSourceError;
use git2::{FetchOptions, RemoteCallbacks, Repository};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Builder for fetching and preparing git repositories
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepositoryBuilder {
	remote_url: String,
	parent_dir: Option<PathBuf>,
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

		// Always try to ensure the repository is available
		// This handles both initial cloning and updating existing repositories
		let mut callbacks = RemoteCallbacks::new();
		callbacks.credentials(|_url, _username_from_url, _allowed_types| git2::Cred::default());

		let mut fetch_options = FetchOptions::new();
		fetch_options.remote_callbacks(callbacks);

		for _ in 0..10 {
			match Repository::clone(&self.remote_url, &repo_path) {
				Ok(repo) => {
					// After cloning, fetch common branches to ensure we have basic coverage
					if let Ok(mut remote) = repo.find_remote("origin") {
						let common_branches = [
							"refs/heads/main:refs/remotes/origin/main",
							"refs/heads/master:refs/remotes/origin/master",
						];
						let _ = remote.fetch(&common_branches, Some(&mut fetch_options), None);
					}
				}
				Err(e) => {
					// If clone fails due to directory existing, that's fine - we'll use the existing repo
					if e.code() == git2::ErrorCode::Exists
						&& e.message().contains("exists and is not an empty directory")
					{
						// Repository already exists, continue
					} else {
						return Err(GitSourceError::Git(e));
					}
				}
			}
		}

		self.with_retry(|| Self::update_existing_repository(&repo_path, &self.remote_url))?;

		// Verify the repository is accessible
		if let Err(e) = Repository::open(&repo_path) {
			return Err(GitSourceError::Git(e));
		}

		Ok(RepositoryManager::new(repo_path))
	}

	/// Update an existing repository (best-effort operation)
	fn update_existing_repository(
		repo_path: &Path,
		remote_url: &str,
	) -> Result<(), GitSourceError> {
		// Try to open the repository - if it fails, that's okay, we'll handle it later
		let repo = match Repository::open(repo_path) {
			Ok(repo) => repo,
			Err(_) => return Ok(()), // Repository not accessible, skip update
		};

		// Try to fetch latest changes - if it fails, that's okay too
		let _ = Self::fetch_latest_changes(&repo, remote_url);
		Ok(())
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
