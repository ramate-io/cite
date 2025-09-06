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
	/// Create a new repository builder
	pub fn new(repository: super::Repository, lock_file: super::lock::LockFile) -> Self {
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

	/// Fetch the repo and the revisions
	pub(crate) fn fetch(&mut self) -> Result<(), GitSourceError> {
		let revisions = self.revisions.clone();
		let mut repository_writer = self.locked_repository_mut().write()?;
		let _ = repository_writer.fetch_and_ensure_trees_for_revisions(&revisions);

		Ok(())
	}
}
