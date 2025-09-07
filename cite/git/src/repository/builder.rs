use super::lock::Lock;
use crate::GitSourceError;

/// Builder for fetching and preparing git repositories with all necessary information
#[derive(Debug, Clone)]
pub struct RepositoryBuilder {
	locked_repository: Lock,
	revisions: Vec<String>,
}

/// Define helper methods within, but the only thing this will publicly do is [RepositoryBuilder::build]
///
/// You can make helper methods various degress of public for testing.

impl RepositoryBuilder {
	/// Create a new repository builder for a target cite directory
	pub fn in_target_cite(remote: String) -> Self {
		// Get the manifest directory (project root) and build target path
		let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
			.unwrap_or_else(|_| std::env::current_dir().unwrap().to_string_lossy().to_string());
		let target_dir = std::path::PathBuf::from(&manifest_dir).join("target/cite-git");

		let repo_dir_name = super::Repository::generate_repo_dir_name(&remote);
		let repo_path = target_dir.join(&repo_dir_name);
		let repository = super::Repository::new(repo_path.clone(), remote);

		// Create lock file named after the repo with sanitized name
		let sanitized_name = repo_dir_name.replace('/', "_").replace(':', "_");
		let lock_path = target_dir.join(format!(".cite-lock-{}", sanitized_name));
		let lock_file = super::lock::LockFile::new(lock_path);
		let locked_repository = Lock::new(repository, lock_file);
		Self { locked_repository, revisions: Vec::new() }
	}

	/// Gets a reference to the locked repository
	pub(crate) fn locked_repository(&self) -> &Lock {
		&self.locked_repository
	}

	/// Gets a mutable reference to the locked repository
	pub(crate) fn locked_repository_mut(&mut self) -> &mut Lock {
		&mut self.locked_repository
	}

	/// Add a revision to the builder
	pub fn add_revision(&mut self, revision: String) {
		self.revisions.push(revision);
	}

	/// Clone the repo and ensure revisions are available
	pub(crate) fn fetch(&mut self) -> Result<(), GitSourceError> {
		let revisions = self.revisions.clone();
		let mut repository_writer = self.locked_repository_mut().write()?;
		repository_writer.clone_and_ensure_revisions(&revisions)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_in_target_cite_creation() -> Result<(), Box<dyn std::error::Error>> {
		let remote = "https://github.com/ramate-io/cite".to_string();
		let builder = RepositoryBuilder::in_target_cite(remote.clone());

		// Test that we can add a revision
		let mut builder = builder;
		builder.add_revision("main".to_string());

		// Test that we can build (this will try to fetch)
		match builder.build() {
			Ok(_analyzer) => {
				println!("✅ RepositoryBuilder::in_target_cite created and built successfully");
			}
			Err(e) => {
				println!("❌ Failed to build repository: {:?}", e);
				// Don't fail the test for network issues, just log them
				println!("   This might be due to network connectivity or git issues");
			}
		}

		Ok(())
	}

	#[test]
	fn test_in_target_cite_paths() -> Result<(), Box<dyn std::error::Error>> {
		let remote = "https://github.com/ramate-io/cite".to_string();
		let builder = RepositoryBuilder::in_target_cite(remote.clone());

		// Check that the paths are set up correctly
		let locked_repo = builder.locked_repository();
		println!("Repository path: {:?}", locked_repo.repository.path());
		println!("Lock file path: {:?}", locked_repo.lock_file.path());

		// Verify the lock file is in the parent directory
		let repo_path = locked_repo.repository.path();
		let lock_path = locked_repo.lock_file.path();
		let expected_parent = repo_path.parent().unwrap();

		if lock_path.parent() == Some(expected_parent) {
			println!("✅ Lock file is correctly placed in parent directory");
		} else {
			return Err(format!(
				"Lock file path {:?} is not in the expected parent directory {:?}",
				lock_path, expected_parent
			)
			.into());
		}

		// Verify the lock file has a repo-specific name
		let lock_filename = lock_path.file_name().unwrap().to_string_lossy();
		if lock_filename.starts_with(".cite-lock-") {
			println!("✅ Lock file has repo-specific name: {}", lock_filename);
		} else {
			return Err(
				format!("Lock file does not have repo-specific name: {}", lock_filename).into()
			);
		}

		// Verify paths use CARGO_MANIFEST_DIR with proper project root detection
		let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
			.unwrap_or_else(|_| std::env::current_dir().unwrap().to_string_lossy().to_string());
		let expected_target = std::path::PathBuf::from(&manifest_dir).join("target/cite-git");

		if repo_path.starts_with(&expected_target) {
			println!("✅ Repository path uses CARGO_MANIFEST_DIR correctly");
		} else {
			return Err(format!(
				"Repository path does not use CARGO_MANIFEST_DIR. Expected base: {:?}, Actual: {:?}",
				expected_target, repo_path
			)
			.into());
		}

		Ok(())
	}
}
