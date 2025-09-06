pub mod lock_file;
pub mod read_guard;
pub mod write_guard;

pub(crate) use lock_file::LockFile;
pub(crate) use read_guard::ReadGuard;
pub(crate) use write_guard::WriteGuard;

use super::Repository;
use crate::GitSourceError;

#[derive(Debug, Clone)]
pub(crate) struct Lock {
	pub repository: Repository,
	pub lock_file: LockFile,
}

impl Lock {
	pub fn new(repository: Repository, lock_file: LockFile) -> Self {
		Self { repository, lock_file }
	}

	pub fn read(&self) -> Result<ReadGuard, GitSourceError> {
		let lock = self.lock_file.read()?;
		Ok(ReadGuard::new(&self.repository, &self.lock_file, lock))
	}

	pub fn write(&mut self) -> Result<WriteGuard, GitSourceError> {
		let lock = self.lock_file.write()?;
		Ok(WriteGuard::new(&mut self.repository, &mut self.lock_file, lock))
	}
}
