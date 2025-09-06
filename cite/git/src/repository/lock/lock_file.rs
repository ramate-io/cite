use file_lock::{FileLock, FileOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::GitSourceError;

/// Identifies then location and the content of the lock file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LockFile {
	path: PathBuf,
	lifetime_writes: u32,
}

/// Lock file is just a separation of concerns to declare the location of the lock file and tie its usage to the borrow checker
impl LockFile {
	/// Create a new lock file at the specified path
	pub fn new(path: PathBuf) -> Self {
		Self { path, lifetime_writes: 0 }
	}
	/// Read the lock file.
	///
	/// When used in the lock struct, this implies obtaining a read lock on the repository directory.
	pub(crate) fn read(&self) -> Result<FileLock, GitSourceError> {
		// read and create if it doesn't exist
		let options = FileOptions::new().read(true).create(true);

		// lock is always blocking because this is for builds which don't have concurrency
		FileLock::lock(self.path.clone(), true, options).map_err(GitSourceError::CreateLockFile)
	}

	/// Write the lock file
	///
	/// When used in the lock struct, this implies obtaining a write lock on the repository directory.
	pub(crate) fn write(&mut self) -> Result<FileLock, GitSourceError> {
		// This ties the lock file borrow into the borrow checker.
		// It is not behaviorally significant, but can be useful for debugging.
		self.lifetime_writes += 1;

		// write and create if it doesn't exist
		let options = FileOptions::new().write(true).create(true);

		// lock is always blocking because this is for builds which don't have concurrency
		FileLock::lock(self.path.clone(), true, options).map_err(GitSourceError::CreateLockFile)
	}
}
