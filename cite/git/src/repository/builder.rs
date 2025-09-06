use crate::GitSourceError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{Repository, RepositoryManager};

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
		Repository::get_target_dir(self.parent_dir.clone())
	}

	/// Generate a simple directory name for a repository
	pub fn generate_repo_dir_name(remote_url: &str) -> String {
		Repository::generate_repo_dir_name(remote_url)
	}

	/// Fetch the repository and return a RepositoryManager
	pub fn fetch(self) -> Result<RepositoryManager, GitSourceError> {
		let target_dir = self.get_target_dir()?;
		let repo_dir_name = Self::generate_repo_dir_name(&self.remote_url);
		let repo_path = target_dir.join(repo_dir_name);

		// Try to clone the repository
		let repository = Repository::clone_from_remote(&self.remote_url, repo_path)?;

		// Create the repository manager with the cloned repository
		RepositoryManager::from_repository(repository)
	}
}
