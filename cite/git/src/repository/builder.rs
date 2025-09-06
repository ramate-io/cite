use super::lock::Lock;
use crate::GitSourceError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{Repository, RepositoryManager};

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
		let repository_writer = self.locked_repository().write()?;
		repository_writer.fetch_and_ensure_trees_for_revisions(&self.revisions);

		Ok(())
	}
}
