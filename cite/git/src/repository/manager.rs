use super::builder::RepositoryBuilder;
use super::lock::Lock;
use crate::GitSourceError;
use std::path::PathBuf;

/// Wraps up read operations, can only be constructed from the [RepositoryBuilder].
#[derive(Debug, Clone)]
pub struct RepositoryAnalyzer {
	/// Semantically, this is a builder that has been built.
	builder: RepositoryBuilder,
}

impl RepositoryAnalyzer {
	// all we need here are the methods to enable content diffs
	// These should largely be refactored up from the content diff and into
	// The [Respository] helper which is behind the guard.
	// That way the concerns for mutability of operations are preserved.

	pub fn get_content_diff_buffer(
		&self,
		referenced: &str,
		current: &str,
	) -> Result<Vec<String>, GitSourceError> {
		let repository_reader = self.builder.locked_repository().read()?;

        // just call directly on the repository reader
        repository_reader.get_content_diff_buffer(referenced, current)

		todo!()
	}
}

impl RepositoryBuilder {
	pub fn build(self) -> Result<RepositoryAnalyzer, GitSourceError> {
		// do all of the fetching that's involved

		Ok(RepositoryAnalyzer { builder: self })
	}
}
