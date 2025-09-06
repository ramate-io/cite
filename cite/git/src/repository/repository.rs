use crate::GitSourceError;
use git2::{FetchOptions, RemoteCallbacks, Repository as Git2Repository};
use std::path::PathBuf;

/// Repository unifies all operations against a repository.
///
/// Unlike a [Git2Repository], [Repository] does not construct on a local repository.
/// Instead it describes an abstract reference to a repository and its intend file location.
#[derive(Debug, Clone)]
pub struct Repository {
	path: PathBuf,
	remote: String,
}

impl Repository {
	/// Create a new repository wrapper
	pub fn new(repo_path: PathBuf) -> Result<Self, GitSourceError> {
		let git_repo = Git2Repository::open(&repo_path).map_err(|e| GitSourceError::Git(e))?;
		Ok(Self { repo_path, git_repo })
	}

	/// Create a new repository by cloning from a remote URL
	pub fn clone_from_remote(remote_url: &str, repo_path: PathBuf) -> Result<Self, GitSourceError> {
		// Remove existing directory if it exists to ensure clean state
		if repo_path.exists() {
			// Try to remove the directory, but don't fail if it's locked
			let _ = std::fs::remove_dir_all(&repo_path);
			// If removal failed, try to remove just the .git directory
			let git_dir = repo_path.join(".git");
			if git_dir.exists() {
				let _ = std::fs::remove_dir_all(&git_dir);
			}
		}

		// Clone the repository with full history to ensure we can access any revision
		// Try multiple times in case of temporary issues
		let mut clone_success = false;
		for attempt in 0..3 {
			let clone_output = std::process::Command::new("git")
				.args(&["clone", remote_url])
				.arg(&repo_path)
				.output()
				.map_err(|e| {
					GitSourceError::InvalidRemote(format!("Failed to run git clone: {}", e))
				})?;

			if clone_output.status.success() {
				clone_success = true;
				break;
			}

			// If this is not the last attempt, clean up and try again
			if attempt < 2 {
				let _ = std::fs::remove_dir_all(&repo_path);
				std::thread::sleep(std::time::Duration::from_millis(100));
			}
		}

		if !clone_success {
			return Err(GitSourceError::InvalidRemote(format!(
				"Failed to clone repository after 3 attempts: {}",
				remote_url
			)));
		}

		// Verify the repository is accessible
		if !repo_path.exists() {
			return Err(GitSourceError::InvalidRemote(
				"Repository was not created successfully".to_string(),
			));
		}

		// Open the cloned repository
		let git_repo = Git2Repository::open(&repo_path).map_err(|e| GitSourceError::Git(e))?;
		Ok(Self { repo_path, git_repo })
	}

	/// Create a new repository by cloning and checking out a specific revision
	pub fn clone_revision(
		remote_url: &str,
		revision: &str,
		repo_path: PathBuf,
	) -> Result<Self, GitSourceError> {
		// First clone the repository
		let repo = Self::clone_from_remote(remote_url, repo_path)?;

		// Then checkout the specific revision
		let checkout_output = std::process::Command::new("git")
			.args(&["-C", &repo.repo_path.to_string_lossy(), "checkout", revision])
			.output()
			.map_err(|e| {
				GitSourceError::InvalidRemote(format!("Failed to run git checkout: {}", e))
			})?;

		if !checkout_output.status.success() {
			return Err(GitSourceError::InvalidRemote(format!(
				"Failed to checkout revision {}: {}",
				revision,
				String::from_utf8_lossy(&checkout_output.stderr)
			)));
		}

		Ok(repo)
	}

	/// Update an existing repository (best-effort operation)
	pub fn update_from_remote(&mut self, remote_url: &str) -> Result<(), GitSourceError> {
		// Try to fetch latest changes - if it fails, that's okay too
		let _ = Self::fetch_latest_changes(&self.git_repo, remote_url);
		Ok(())
	}

	/// Fetch latest changes for an existing repository
	fn fetch_latest_changes(repo: &Git2Repository, remote_url: &str) -> Result<(), GitSourceError> {
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
		&self.repo_path
	}

	/// Get the underlying git2 repository (read-only access)
	pub fn git_repo(&self) -> &Git2Repository {
		&self.git_repo
	}

	/// Check if a revision exists in the repository
	pub fn revision_exists(&self, revision: &str) -> bool {
		// First try the revision as-is
		if self.git_repo.revparse_single(revision).is_ok() {
			return true;
		}

		// If it's a branch name, try common branch reference patterns
		if !revision.starts_with("refs/") && !revision.chars().all(|c| c.is_ascii_hexdigit()) {
			// Try origin/branch pattern
			if self.git_repo.revparse_single(&format!("origin/{}", revision)).is_ok() {
				return true;
			}

			// Try refs/heads/branch pattern
			if self.git_repo.revparse_single(&format!("refs/heads/{}", revision)).is_ok() {
				return true;
			}

			// Try refs/remotes/origin/branch pattern
			if self
				.git_repo
				.revparse_single(&format!("refs/remotes/origin/{}", revision))
				.is_ok()
			{
				return true;
			}
		}

		false
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

	/// Fetch specific revisions that are needed (mutable operation)
	pub fn fetch_specific_revisions(&mut self, revisions: &[&str]) -> Result<(), GitSourceError> {
		let mut remote = self.git_repo.find_remote("origin").map_err(|e| GitSourceError::Git(e))?;

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
			let fetch_result = remote.fetch(&revisions_to_fetch, Some(&mut fetch_options), None);

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
				if let Ok(obj) = self.git_repo.revparse_single(revision) {
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
							let _ = self.git_repo.revparse_single(revision);
						}
					}
				}
			}
		}

		Ok(())
	}
}
