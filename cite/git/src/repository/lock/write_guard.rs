use super::{super::Repository, LockFile};
use file_lock::FileLock;
use std::ops::{Deref, DerefMut};

pub struct WriteGuard<'a> {
	_repository: &'a mut Repository,
	_lock_file: &'a mut LockFile, // Keeps the lock alive for lifetime
	_lock: FileLock,
}

impl<'a> WriteGuard<'a> {
	pub(crate) fn new(
		repository: &'a mut Repository,
		lock_file: &'a mut LockFile,
		lock: FileLock,
	) -> Self {
		Self { _repository: repository, _lock_file: lock_file, _lock: lock }
	}
}

// Required by DerefMut, but can be minimal
impl<'a> Deref for WriteGuard<'a> {
	type Target = Repository;

	fn deref(&self) -> &Self::Target {
		self._repository
	}
}

impl<'a> DerefMut for WriteGuard<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self._repository
	}
}
