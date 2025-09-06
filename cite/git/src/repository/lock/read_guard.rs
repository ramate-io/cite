use super::{super::Repository, LockFile};
use file_lock::FileLock;
use std::ops::Deref;

pub struct ReadGuard<'a> {
	_repository: &'a Repository,
	// We keep both the lock file and the for completeness
	_lock_file: &'a LockFile,
	_lock: FileLock,
}

impl<'a> ReadGuard<'a> {
	pub fn new(repository: &'a Repository, lock_file: &'a LockFile, lock: FileLock) -> Self {
		Self { _repository: repository, _lock_file: lock_file, _lock: lock }
	}
}

impl<'a> Deref for ReadGuard<'a> {
	type Target = Repository;

	fn deref(&self) -> &Self::Target {
		&self._repository
	}
}
