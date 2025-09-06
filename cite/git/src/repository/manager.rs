use super::lock::Lock;
use crate::GitSourceError;
use std::path::PathBuf;

/// Manages operations on a cached git repository with advisory locking
#[derive(Debug, Clone)]
pub struct RepositoryManager {
	locked_repository: Lock,
}

impl RepositoryManager {}
