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

/// NOTE: I just took a look at how Cargo is handling git concurrency, and they are not...
///
/// https://github.com/rust-lang/cargo/blob/3ceb2cb2504fed7446be428c3b8715b696161487/src/cargo/sources/git/source.rs#L445
/// The clones are single threaded when constructing from the package manager.
/// Subsequent revision specific activities are performed on locked copies of the repository.
impl Lock {
	pub fn new(repository: Repository, lock_file: LockFile) -> Self {
		Self { repository, lock_file }
	}

	pub fn wait_for_no_git_locks(&self) {
		let repository_path = self.repository.path();

		loop {
			let mut any_locks_found = false;

			// Use glob patterns to find all git lock files
			let patterns = [".git/*.lock", ".git/refs/**/*.lock", ".git/objects/**/*.lock"];

			for pattern in &patterns {
				let full_pattern = repository_path.join(pattern);
				if let Ok(entries) = glob::glob(&full_pattern.to_string_lossy()) {
					for entry in entries {
						if let Ok(_) = entry {
							any_locks_found = true;
							break;
						}
					}
				}
				if any_locks_found {
					break;
				}
			}

			if !any_locks_found {
				// All locks are gone, we can proceed
				break;
			}

			// Wait a bit before checking again
			std::thread::sleep(std::time::Duration::from_millis(100));
		}
	}

	pub fn read(&self) -> Result<ReadGuard, GitSourceError> {
		let lock = self.lock_file.read()?;

		self.wait_for_no_git_locks();

		Ok(ReadGuard::new(&self.repository, &self.lock_file, lock))
	}

	pub fn write(&mut self) -> Result<WriteGuard, GitSourceError> {
		let lock = self.lock_file.write()?;

		self.wait_for_no_git_locks();

		Ok(WriteGuard::new(&mut self.repository, &mut self.lock_file, lock))
	}
}
