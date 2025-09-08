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

	#[cfg(test)]
	/// Get the lock file path
	pub fn path(&self) -> &PathBuf {
		&self.path
	}

	/// Read the lock file.
	///
	/// When used in the lock struct, this implies obtaining a read lock on the repository directory.
	/// This is optimistic - assumes the file already exists from a previous write operation.
	pub(crate) fn read(&self) -> Result<FileLock, GitSourceError> {
		// Read-only lock, no create - assumes file exists from previous write
		let options = FileOptions::new().read(true).write(true); // DEBUG: file lock does not seemd to be working.

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

		// Ensure parent directory exists
		if let Some(parent) = self.path.parent() {
			std::fs::create_dir_all(parent).map_err(GitSourceError::CreateLockFile)?;
		}

		// write and create if it doesn't exist
		let options = FileOptions::new().write(true).create(true);

		// lock is always blocking because this is for builds which don't have concurrency
		let file_lock = FileLock::lock(self.path.clone(), true, options)
			.map_err(GitSourceError::CreateLockFile)?;

		// Write some content to the file (just the path as suggested)
		if let Ok(mut file) = std::fs::File::create(&self.path) {
			let _ = std::io::Write::write_all(&mut file, self.path.to_string_lossy().as_bytes());
		}

		Ok(file_lock)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_lock_file_creation() -> Result<(), Box<dyn std::error::Error>> {
		let temp_dir = std::env::temp_dir().join("cite-test-lock");
		let lock_path = temp_dir.join(".cite-lock");

		// Clean up any existing test directory
		let _ = std::fs::remove_dir_all(&temp_dir);

		let mut lock_file = LockFile::new(lock_path.clone());

		// First create the file with a write lock
		match lock_file.write() {
			Ok(_file_lock) => {
				println!("✅ Write lock created successfully at: {:?}", lock_path);
			}
			Err(e) => {
				println!("❌ Failed to create write lock: {:?}", e);
				return Err(format!("Write lock creation failed: {}", e).into());
			}
		}

		// Now test read lock creation (should work since file exists)
		match lock_file.read() {
			Ok(_file_lock) => {
				println!("✅ Read lock created successfully at: {:?}", lock_path);
			}
			Err(e) => {
				println!("❌ Failed to create read lock: {:?}", e);
				println!("   Path: {:?}", lock_path);
				println!("   Parent exists: {:?}", lock_path.parent().map(|p| p.exists()));
				println!("   Parent: {:?}", lock_path.parent());
				return Err(format!("Read lock creation failed: {}", e).into());
			}
		}

		// Clean up
		let _ = std::fs::remove_dir_all(&temp_dir);
		Ok(())
	}

	#[test]
	fn test_lock_file_write() -> Result<(), Box<dyn std::error::Error>> {
		let temp_dir = std::env::temp_dir().join("cite-test-lock-write");
		let lock_path = temp_dir.join(".cite-lock");

		// Clean up any existing test directory
		let _ = std::fs::remove_dir_all(&temp_dir);

		let mut lock_file = LockFile::new(lock_path.clone());

		// Test write lock creation
		match lock_file.write() {
			Ok(_file_lock) => {
				println!("✅ Write lock created successfully at: {:?}", lock_path);
			}
			Err(e) => {
				println!("❌ Failed to create write lock: {:?}", e);
				println!("   Path: {:?}", lock_path);
				println!("   Parent exists: {:?}", lock_path.parent().map(|p| p.exists()));
				println!("   Parent: {:?}", lock_path.parent());
				return Err(format!("Write lock creation failed: {}", e).into());
			}
		}

		// Clean up
		let _ = std::fs::remove_dir_all(&temp_dir);
		Ok(())
	}

	#[test]
	fn test_target_cite_paths() -> Result<(), Box<dyn std::error::Error>> {
		let remote = "https://github.com/ramate-io/cite".to_string();

		// Use the same logic as RepositoryBuilder::in_target_cite
		let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
			.unwrap_or_else(|_| std::env::current_dir().unwrap().to_string_lossy().to_string());
		let target_dir = std::path::PathBuf::from(&manifest_dir).join("target/cite-git");

		let repo_dir_name = super::super::Repository::generate_repo_dir_name(&remote);
		let repo_path = target_dir.join(&repo_dir_name);

		// Create lock file named after the repo with sanitized name
		let sanitized_name = repo_dir_name.replace('/', "_").replace(':', "_");
		let lock_path = target_dir.join(format!(".cite-lock-{}", sanitized_name));

		println!("Repo path: {:?}", repo_path);
		println!("Lock path: {:?}", lock_path);
		println!("Lock parent: {:?}", lock_path.parent());
		println!("Lock parent exists: {:?}", lock_path.parent().map(|p| p.exists()));
		println!("Sanitized name: {}", sanitized_name);

		// Test that we can create the lock file at this path
		let mut lock_file = LockFile::new(lock_path.clone());

		// First create with write lock
		match lock_file.write() {
			Ok(_file_lock) => {
				println!("✅ Target cite write lock created successfully");
			}
			Err(e) => {
				println!("❌ Failed to create target cite write lock: {:?}", e);
				return Err(format!("Target cite write lock creation failed: {}", e).into());
			}
		}

		// Then test read lock
		match lock_file.read() {
			Ok(_file_lock) => {
				println!("✅ Target cite read lock created successfully");
			}
			Err(e) => {
				println!("❌ Failed to create target cite read lock: {:?}", e);
				return Err(format!("Target cite read lock creation failed: {}", e).into());
			}
		}

		// Verify the lock file has the correct repo-specific name
		let lock_filename = lock_path.file_name().unwrap().to_string_lossy();
		if lock_filename.starts_with(".cite-lock-") {
			println!("✅ Lock file has repo-specific name: {}", lock_filename);
		} else {
			return Err(
				format!("Lock file does not have repo-specific name: {}", lock_filename).into()
			);
		}

		Ok(())
	}

	#[test]
	fn test_lock_file_with_nonexistent_parent() -> Result<(), Box<dyn std::error::Error>> {
		let temp_dir = std::env::temp_dir().join("cite-test-nonexistent-parent");
		let lock_path = temp_dir.join("nonexistent").join(".cite-lock");

		// Clean up any existing test directory
		let _ = std::fs::remove_dir_all(&temp_dir);

		let mut lock_file = LockFile::new(lock_path.clone());

		// Test that our automatic parent creation works with write lock
		match lock_file.write() {
			Ok(_file_lock) => {
				println!(
					"✅ Write lock created successfully with nonexistent parent at: {:?}",
					lock_path
				);
			}
			Err(e) => {
				println!("❌ Failed to create write lock with nonexistent parent: {:?}", e);
				println!("   Path: {:?}", lock_path);
				println!("   Parent exists: {:?}", lock_path.parent().map(|p| p.exists()));
				println!("   Parent: {:?}", lock_path.parent());
				return Err(
					format!("Write lock creation with nonexistent parent failed: {}", e).into()
				);
			}
		}

		// Now test read lock (should work since file exists)
		match lock_file.read() {
			Ok(_file_lock) => {
				println!(
					"✅ Read lock created successfully with nonexistent parent at: {:?}",
					lock_path
				);
			}
			Err(e) => {
				println!("❌ Failed to create read lock with nonexistent parent: {:?}", e);
				return Err(
					format!("Read lock creation with nonexistent parent failed: {}", e).into()
				);
			}
		}

		// Clean up
		let _ = std::fs::remove_dir_all(&temp_dir);
		Ok(())
	}
}
