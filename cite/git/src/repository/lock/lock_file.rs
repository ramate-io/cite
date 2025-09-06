use file_lock::{FileLock, FileOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::GitSourceError;

/// Identifies then location and the content of the lock file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LockFile {
	path: PathBuf,
	is_writing: bool,
}

/// Lock file is just a separation of concerns to declare the location of the lock file and tie its usage to the borrow checker
impl LockFile {
	/// Read the lock file
	///
	/// This is meant to be embedded in the repository manager, which will make a meaningful guard
	pub(crate) fn read(&self) -> Result<FileLock, GitSourceError> {
		// read and create if it doesn't exist
		let options = FileOptions::new().read(true).create(true);

		// lock is always blocking because this is for builds which don't have concurrency
		FileLock::lock(self.path.clone(), true, options).map_err(GitSourceError::CreateLockFile)
	}

	/// Write the lock file
	///
	/// This is meant to be embedded in the repository manager, which will make a meaningful guard
	pub(crate) fn write(&mut self) -> Result<FileLock, GitSourceError> {
		// this ties the lock file borrow into the borrow checker
		// Additionally, because we do not expose unlock, it will in fact be true that we are writing until the lock file is dropped
		self.is_writing = true;

		// write and create if it doesn't exist
		let options = FileOptions::new().write(true).create(true);

		// lock is always blocking because this is for builds which don't have concurrency
		FileLock::lock(self.path.clone(), true, options).map_err(GitSourceError::CreateLockFile)
	}
}
