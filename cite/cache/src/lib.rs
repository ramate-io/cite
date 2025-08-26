use cite_core::id::Id;
use cite_core::{Comparison, Current, Diff, Referenced, Source, SourceError};
use std::path::PathBuf;

/// A cachable reference is serializable and deserializable
///
/// The reference only needs to read from the cache file.
pub trait CacheableReferenced: Referenced + Sized {
	fn from_cached_buffer(buffer: Vec<u8>) -> Result<Self, CacheError>;
}

/// A cacheable current is serializable and deserializable
///
/// The current needs to be able to write to the cache file.
pub trait CacheableCurrent<R: CacheableReferenced, D: Diff>: Current<R, D> + Sized {
	fn to_cached_buffer(&self) -> Result<Vec<u8>, CacheError>;
}

/// Errors thrown by the [CacheBuilder].
#[derive(Debug, thiserror::Error)]
pub enum CacheBuilderError {
	#[error("Failed to create cache directory: {0}")]
	CreateCacheDir(#[source] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct CacheBuilder {
	pub cite_dir: PathBuf,
	pub cache_subdir: PathBuf,
}

impl CacheBuilder {
	pub fn new(cite_dir: PathBuf, cache_subdir: PathBuf) -> Self {
		Self { cite_dir, cache_subdir }
	}

	pub fn build(&self) -> Result<Cache, CacheBuilderError> {
		let cache_dir = self.cite_dir.join(self.cache_subdir.clone());
		std::fs::create_dir_all(&cache_dir).map_err(CacheBuilderError::CreateCacheDir)?;
		Ok(Cache { builder: self.clone() })
	}

	/// Tries to create a [CacheBuilder] from the workspace root.
	///
	/// If there is no workspace, root, uses the default [CacheBuilder].
	pub fn try_canonical() -> Result<Self, CacheBuilderError> {
		match cargo_metadata::MetadataCommand::new().exec() {
			Ok(metadata) => {
				let cite_dir = metadata.workspace_root.join(".cite").into();
				let cache_subdir = metadata.workspace_root.join("cache").into();
				Ok(Self { cite_dir, cache_subdir })
			}
			Err(_) => Ok(Self::default()),
		}
	}
}

impl Default for CacheBuilder {
	fn default() -> Self {
		Self::new(PathBuf::from(".cite"), PathBuf::from("cache"))
	}
}

/// Errors thrown by the [Cache].
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
	#[error("Failed to read cache file: {0}")]
	ReadCacheFile(#[source] std::io::Error),

	#[error("Failed to write cache file: {0}")]
	WriteCacheFile(#[source] std::io::Error),

	#[error("Cache file not found: {0}")]
	CacheFileNotFound(#[source] std::io::Error),

	#[error("Failed to delete cache file: {0}")]
	DeleteCacheFile(#[source] std::io::Error),

	#[error("Failed to serialize cacheable: {0}")]
	Serialize(#[source] Box<dyn std::error::Error + Send + Sync>),

	#[error("Failed to deserialize cacheable: {0}")]
	Deserialize(#[source] Box<dyn std::error::Error + Send + Sync>),

	#[error("Source error: {0}")]
	SourceError(#[source] SourceError),
}

#[derive(Debug, Clone)]
pub struct Cache {
	builder: CacheBuilder,
}

#[derive(Debug, Clone)]
pub enum CacheBehavior {
	Enabled,
	Ignored,
}

impl Cache {
	pub fn cite_dir(&self) -> &PathBuf {
		&self.builder.cite_dir
	}

	pub fn cache_subdir(&self) -> &PathBuf {
		&self.builder.cache_subdir
	}

	pub fn cache_dir(&self) -> PathBuf {
		self.builder.cite_dir.join(self.builder.cache_subdir.clone())
	}

	pub fn get<R: CacheableReferenced>(&self, id: &Id) -> Result<Option<R>, CacheError> {
		let cache_file = self.cache_dir().join(id.as_str());
		if !cache_file.exists() {
			return Ok(None);
		}
		let cache_buffer =
			std::fs::read_to_string(&cache_file).map_err(CacheError::ReadCacheFile)?;
		let cached_entry = R::from_cached_buffer(cache_buffer.as_bytes().to_vec())?;
		Ok(Some(cached_entry))
	}

	pub fn set<R: CacheableReferenced, C: CacheableCurrent<R, D>, D: Diff>(
		&self,
		id: &Id,
		value: &C,
	) -> Result<(), CacheError> {
		let cache_file = self.cache_dir().join(id.as_str());
		let cache_buffer = value.to_cached_buffer()?;
		std::fs::write(&cache_file, cache_buffer).map_err(CacheError::WriteCacheFile)?;
		Ok(())
	}

	pub fn delete(&self, id: &Id) -> Result<(), CacheError> {
		let cache_file = self.cache_dir().join(id.as_str());
		if !cache_file.exists() {
			return Err(CacheError::CacheFileNotFound(std::io::Error::new(
				std::io::ErrorKind::NotFound,
				format!("Cache file not found: {}", cache_file.display()),
			)));
		}
		std::fs::remove_file(&cache_file).map_err(CacheError::DeleteCacheFile)?;
		Ok(())
	}

	/// Get a source with cache.
	///
	/// If the cache is ignored, the source is fetched via [Source::get] and the cache is filled with the current value.
	///
	/// If the cache is enabled, we first check if the source is in the cache.
	/// If it is, we use the cached value.
	/// If it is not, we fetch the source via [Source::get_referenced] and [Source::get_current] and fill the cache with the current value.
	///
	/// Note: this caching discprenacy between referenced and current means that a source that does not have a reference and current implementation that serialize to the same thing for the same content may always return a diff.
	pub fn get_source_with_cache<
		S: Source<R, C, D>,
		R: CacheableReferenced,
		C: CacheableCurrent<R, D>,
		D: Diff,
	>(
		&self,
		source: &S,
		behavior: CacheBehavior,
	) -> Result<Comparison<R, C, D>, CacheError> {
		match behavior {
			CacheBehavior::Ignored => {
				let comparison = source.get().map_err(CacheError::SourceError)?;
				self.set(source.id(), comparison.current())?;
				Ok(comparison)
			}
			CacheBehavior::Enabled => {
				let (referenced, current) = match self.get::<R>(source.id())? {
					Some(referenced) => {
						(referenced, source.get_current().map_err(CacheError::SourceError)?)
					}
					None => {
						let referenced =
							source.get_referenced().map_err(CacheError::SourceError)?;
						let current = source.get_current().map_err(CacheError::SourceError)?;
						self.set(source.id(), &current)?;
						(referenced, current)
					}
				};
				let diff = current.diff(&referenced).map_err(CacheError::SourceError)?;
				let comparison = Comparison::new(referenced, current, diff);
				Ok(comparison)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use cite_core::Content;
	use serde::{Deserialize, Serialize};
	use tempfile::TempDir;

	// Test implementations for CacheableReferenced and CacheableCurrent
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct TestReferenced {
		content: String,
	}

	impl Content for TestReferenced {}
	impl Referenced for TestReferenced {}
	impl CacheableReferenced for TestReferenced {
		fn from_cached_buffer(buffer: Vec<u8>) -> Result<Self, CacheError> {
			let content =
				String::from_utf8(buffer).map_err(|e| CacheError::Deserialize(e.into()))?;
			Ok(Self { content })
		}
	}

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct TestCurrent {
		content: String,
	}

	impl Content for TestCurrent {}

	impl Current<TestReferenced, TestDiff> for TestCurrent {
		fn diff(&self, referenced: &TestReferenced) -> Result<TestDiff, SourceError> {
			Ok(TestDiff { changed: self.content != referenced.content })
		}
	}

	impl CacheableCurrent<TestReferenced, TestDiff> for TestCurrent {
		fn to_cached_buffer(&self) -> Result<Vec<u8>, CacheError> {
			// content to utf8
			let buffer = self.content.as_bytes().to_vec();
			Ok(buffer)
		}
	}

	#[derive(Debug, Clone, PartialEq)]
	struct TestDiff {
		changed: bool,
	}

	impl Diff for TestDiff {
		fn is_empty(&self) -> bool {
			!self.changed
		}
	}

	struct TestSource {
		id: Id,
		referenced: TestReferenced,
		current: TestCurrent,
	}

	impl Source<TestReferenced, TestCurrent, TestDiff> for TestSource {
		fn id(&self) -> &Id {
			&self.id
		}

		fn get(&self) -> Result<Comparison<TestReferenced, TestCurrent, TestDiff>, SourceError> {
			let diff = self.current.diff(&self.referenced)?;
			Ok(Comparison::new(self.referenced.clone(), self.current.clone(), diff))
		}

		fn get_referenced(&self) -> Result<TestReferenced, SourceError> {
			Ok(self.referenced.clone())
		}

		fn get_current(&self) -> Result<TestCurrent, SourceError> {
			Ok(self.current.clone())
		}
	}

	#[test]
	fn test_cache_builder_default() {
		let builder = CacheBuilder::default();
		assert_eq!(builder.cite_dir, PathBuf::from(".cite"));
		assert_eq!(builder.cache_subdir, PathBuf::from("cache"));
	}

	#[test]
	fn test_cache_builder_custom() {
		let cite_dir = PathBuf::from("/tmp/test-cite");
		let cache_subdir = PathBuf::from("custom-cache");
		let builder = CacheBuilder::new(cite_dir.clone(), cache_subdir.clone());

		assert_eq!(builder.cite_dir, cite_dir);
		assert_eq!(builder.cache_subdir, cache_subdir);
	}

	#[test]
	fn test_cache_build() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let cite_dir = temp_dir.path().join("cite");
		let cache_subdir = PathBuf::from("cache");

		let builder = CacheBuilder::new(cite_dir.clone(), cache_subdir.clone());
		let cache = builder.build()?;

		assert_eq!(cache.cite_dir(), &cite_dir);
		assert_eq!(cache.cache_subdir(), &cache_subdir);
		assert_eq!(cache.cache_dir(), cite_dir.join(cache_subdir));
		assert!(cache.cache_dir().exists());
		Ok(())
	}

	#[test]
	fn test_cache_get_set_delete() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let id = Id::new("test-key".to_string());
		let _referenced = TestReferenced { content: "test content".to_string() };

		// Test that cache is initially empty
		let result = cache.get::<TestReferenced>(&id)?;
		assert!(result.is_none());

		// Test set
		let current = TestCurrent { content: "test content".to_string() };
		cache.set(&id, &current)?;

		// Test get after set
		let result = cache.get::<TestReferenced>(&id)?;
		assert!(result.is_some());
		if let Some(retrieved) = result {
			assert_eq!(retrieved.content, "test content");
		}

		// Test delete
		cache.delete(&id)?;
		let result = cache.get::<TestReferenced>(&id)?;
		assert!(result.is_none());
		Ok(())
	}

	#[test]
	fn test_cacheable_referenced_serialization() -> Result<(), anyhow::Error> {
		let referenced = TestReferenced { content: "test content".to_string() };

		// Simulate what would be cached (using the same format as TestCurrent)
		let serialized = referenced.content.as_bytes().to_vec();
		let deserialized = TestReferenced::from_cached_buffer(serialized)?;

		assert_eq!(referenced, deserialized);
		Ok(())
	}

	#[test]
	fn test_cacheable_current_serialization() -> Result<(), anyhow::Error> {
		let current = TestCurrent { content: "test content".to_string() };

		let buffer = current.to_cached_buffer()?;
		let deserialized_content = String::from_utf8(buffer.clone())?;
		assert_eq!(current.content, deserialized_content);

		// Test round-trip through TestReferenced (since that's what gets cached)
		let as_referenced = TestReferenced::from_cached_buffer(buffer)?;
		assert_eq!(current.content, as_referenced.content);
		Ok(())
	}

	#[test]
	fn test_get_source_with_cache_ignored() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let source = TestSource {
			id: Id::new("test-source".to_string()),
			referenced: TestReferenced { content: "ref content".to_string() },
			current: TestCurrent { content: "current content".to_string() },
		};

		let result = cache.get_source_with_cache(&source, CacheBehavior::Ignored)?;

		assert_eq!(result.referenced().content, "ref content");
		assert_eq!(result.current().content, "current content");
		assert!(result.diff().changed);
		Ok(())
	}

	#[test]
	fn test_get_source_with_cache_enabled_no_cache() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let source = TestSource {
			id: Id::new("test-source".to_string()),
			referenced: TestReferenced { content: "ref content".to_string() },
			current: TestCurrent { content: "current content".to_string() },
		};

		let result = cache.get_source_with_cache(&source, CacheBehavior::Enabled)?;

		assert_eq!(result.referenced().content, "ref content");
		assert_eq!(result.current().content, "current content");
		assert!(result.diff().changed);
		Ok(())
	}

	#[test]
	fn test_get_source_with_cache_enabled_with_cache() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let id = Id::new("test-source".to_string());

		// Pre-populate cache with different content
		let cached_current = TestCurrent { content: "cached content".to_string() };
		cache.set(&id, &cached_current)?;

		let source = TestSource {
			id: id.clone(),
			referenced: TestReferenced { content: "ref content".to_string() },
			current: TestCurrent { content: "current content".to_string() },
		};

		let result = cache.get_source_with_cache(&source, CacheBehavior::Enabled)?;

		// Should use cached referenced content
		assert_eq!(result.referenced().content, "cached content");
		// Should still fetch fresh current content
		assert_eq!(result.current().content, "current content");
		assert!(result.diff().changed);
		Ok(())
	}

	#[test]
	fn test_cache_serialization_consistency() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let id = Id::new("consistency-test".to_string());
		let test_content = "unified test content";

		// Create both referenced and current with same content
		let original_current = TestCurrent { content: test_content.to_string() };
		let original_referenced = TestReferenced { content: test_content.to_string() };

		// Cache the current value
		cache.set(&id, &original_current)?;

		// Retrieve as referenced (this is what the cache stores)
		let cached_referenced =
			cache.get::<TestReferenced>(&id)?.expect("Should have cached value");

		// Verify that both serialization paths produce compatible results
		assert_eq!(cached_referenced.content, original_current.content);
		assert_eq!(cached_referenced.content, original_referenced.content);

		// Verify the actual serialization formats are compatible
		let current_buffer = original_current.to_cached_buffer()?;
		let referenced_from_buffer = TestReferenced::from_cached_buffer(current_buffer)?;
		assert_eq!(referenced_from_buffer.content, original_current.content);

		Ok(())
	}

	#[test]
	fn test_cache_workflow_complete() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		// Test 1: First access with no cache - should populate cache
		let source1 = TestSource {
			id: Id::new("workflow-test".to_string()),
			referenced: TestReferenced { content: "original referenced".to_string() },
			current: TestCurrent { content: "original current".to_string() },
		};

		let result1 = cache.get_source_with_cache(&source1, CacheBehavior::Enabled)?;
		assert_eq!(result1.referenced().content, "original referenced");
		assert_eq!(result1.current().content, "original current");

		// Test 2: Second access with cache - should use cached referenced, fresh current
		let source2 = TestSource {
			id: Id::new("workflow-test".to_string()),
			referenced: TestReferenced { content: "NEW referenced".to_string() }, // This should be ignored
			current: TestCurrent { content: "NEW current".to_string() },
		};

		let result2 = cache.get_source_with_cache(&source2, CacheBehavior::Enabled)?;
		assert_eq!(result2.referenced().content, "original current"); // Uses cached value
		assert_eq!(result2.current().content, "NEW current"); // Uses fresh value
		assert!(result2.diff().changed); // Should show difference

		Ok(())
	}

	#[test]
	fn test_cache_ignored_populates_cache() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let id = Id::new("ignored-population-test".to_string());

		// Verify cache is initially empty
		assert!(cache.get::<TestReferenced>(&id)?.is_none());

		let source = TestSource {
			id: id.clone(),
			referenced: TestReferenced { content: "ref content".to_string() },
			current: TestCurrent { content: "current content".to_string() },
		};

		// Use cache with Ignored behavior
		let result = cache.get_source_with_cache(&source, CacheBehavior::Ignored)?;

		// Verify the comparison result is correct
		assert_eq!(result.referenced().content, "ref content");
		assert_eq!(result.current().content, "current content");
		assert!(result.diff().changed);

		// Verify that cache was populated with the CURRENT value
		let cached_value = cache.get::<TestReferenced>(&id)?.expect("Cache should be populated");
		assert_eq!(cached_value.content, "current content"); // Should be current, not referenced

		Ok(())
	}

	#[test]
	fn test_cache_enabled_no_cache_populates_with_current() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let id = Id::new("enabled-no-cache-test".to_string());

		// Verify cache is initially empty
		assert!(cache.get::<TestReferenced>(&id)?.is_none());

		let source = TestSource {
			id: id.clone(),
			referenced: TestReferenced { content: "original ref".to_string() },
			current: TestCurrent { content: "original current".to_string() },
		};

		// Use cache with Enabled behavior (no existing cache)
		let result = cache.get_source_with_cache(&source, CacheBehavior::Enabled)?;

		// Should use the source's referenced and current values
		assert_eq!(result.referenced().content, "original ref");
		assert_eq!(result.current().content, "original current");
		assert!(result.diff().changed);

		// Verify that cache was populated with the CURRENT value
		let cached_value = cache.get::<TestReferenced>(&id)?.expect("Cache should be populated");
		assert_eq!(cached_value.content, "original current"); // Should be current, not referenced

		Ok(())
	}

	#[test]
	fn test_cache_enabled_with_existing_cache_uses_cached_referenced() -> Result<(), anyhow::Error>
	{
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let id = Id::new("enabled-with-cache-test".to_string());

		// Pre-populate cache
		let cached_current = TestCurrent { content: "previously cached content".to_string() };
		cache.set(&id, &cached_current)?;

		// Verify cache contains our value
		let cached_referenced =
			cache.get::<TestReferenced>(&id)?.expect("Cache should contain value");
		assert_eq!(cached_referenced.content, "previously cached content");

		let source = TestSource {
			id: id.clone(),
			referenced: TestReferenced { content: "NEW referenced".to_string() }, // Should be ignored
			current: TestCurrent { content: "NEW current".to_string() },
		};

		// Use cache with Enabled behavior (with existing cache)
		let result = cache.get_source_with_cache(&source, CacheBehavior::Enabled)?;

		// Should use cached value for referenced, fresh current
		assert_eq!(result.referenced().content, "previously cached content"); // From cache
		assert_eq!(result.current().content, "NEW current"); // Fresh from source
		assert!(result.diff().changed);

		// Cache should NOT be updated when using existing cache
		let still_cached =
			cache.get::<TestReferenced>(&id)?.expect("Cache should still contain original");
		assert_eq!(still_cached.content, "previously cached content"); // Unchanged

		Ok(())
	}

	#[test]
	fn test_cache_discrepancy_serialization_issue() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let id = Id::new("discrepancy-test".to_string());
		let identical_content = "identical content";

		// Create source where referenced and current have identical content
		let source = TestSource {
			id: id.clone(),
			referenced: TestReferenced { content: identical_content.to_string() },
			current: TestCurrent { content: identical_content.to_string() },
		};

		// First call with Enabled (no cache) - should populate cache with current
		let result1 = cache.get_source_with_cache(&source, CacheBehavior::Enabled)?;
		assert_eq!(result1.referenced().content, identical_content);
		assert_eq!(result1.current().content, identical_content);
		assert!(!result1.diff().changed); // Should be identical, no diff

		// Now create a NEW source with the SAME content
		let source2 = TestSource {
			id: id.clone(),
			referenced: TestReferenced { content: identical_content.to_string() },
			current: TestCurrent { content: identical_content.to_string() },
		};

		// Second call with Enabled (with cache) - demonstrates the discrepancy
		let result2 = cache.get_source_with_cache(&source2, CacheBehavior::Enabled)?;

		// The "discrepancy" mentioned in docs:
		// - referenced comes from cache (which was serialized as current)
		// - current comes fresh from source
		// - Even though content is identical, serialization round-trip might cause differences
		assert_eq!(result2.referenced().content, identical_content); // From cache
		assert_eq!(result2.current().content, identical_content); // Fresh from source

		// In our case, both use the same serialization format (UTF-8 strings),
		// so they should still be equal and no diff should be detected
		assert!(!result2.diff().changed);

		Ok(())
	}

	#[test]
	fn test_cache_behavior_comparison() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		// Test both behaviors with the same source data
		let create_source = |id_suffix: &str| TestSource {
			id: Id::new(format!("comparison-{}", id_suffix)),
			referenced: TestReferenced { content: "ref data".to_string() },
			current: TestCurrent { content: "current data".to_string() },
		};

		// Test Ignored behavior
		let source_ignored = create_source("ignored");
		let id_ignored = source_ignored.id.clone();
		let result_ignored =
			cache.get_source_with_cache(&source_ignored, CacheBehavior::Ignored)?;

		// Test Enabled behavior (no cache)
		let source_enabled = create_source("enabled");
		let id_enabled = source_enabled.id.clone();
		let result_enabled =
			cache.get_source_with_cache(&source_enabled, CacheBehavior::Enabled)?;

		// Both should produce the same comparison results
		assert_eq!(result_ignored.referenced().content, result_enabled.referenced().content);
		assert_eq!(result_ignored.current().content, result_enabled.current().content);
		assert_eq!(result_ignored.diff().changed, result_enabled.diff().changed);

		// Both should have populated cache with current content
		let cached_ignored = cache.get::<TestReferenced>(&id_ignored)?.expect("Should be cached");
		let cached_enabled = cache.get::<TestReferenced>(&id_enabled)?.expect("Should be cached");

		assert_eq!(cached_ignored.content, "current data");
		assert_eq!(cached_enabled.content, "current data");
		assert_eq!(cached_ignored.content, cached_enabled.content);

		Ok(())
	}

	#[test]
	fn test_cache_multiple_updates_enabled_behavior() -> Result<(), anyhow::Error> {
		let temp_dir = TempDir::new()?;
		let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
		let cache = builder.build()?;

		let id = Id::new("multiple-updates".to_string());

		// First source - will populate cache
		let source1 = TestSource {
			id: id.clone(),
			referenced: TestReferenced { content: "first ref".to_string() },
			current: TestCurrent { content: "first current".to_string() },
		};

		let result1 = cache.get_source_with_cache(&source1, CacheBehavior::Enabled)?;
		assert_eq!(result1.referenced().content, "first ref");
		assert_eq!(result1.current().content, "first current");

		// Verify cache was populated
		let cached1 = cache.get::<TestReferenced>(&id)?.expect("Should be cached");
		assert_eq!(cached1.content, "first current");

		// Second source - will use cached referenced, fresh current
		let source2 = TestSource {
			id: id.clone(),
			referenced: TestReferenced { content: "second ref".to_string() }, // Ignored
			current: TestCurrent { content: "second current".to_string() },
		};

		let result2 = cache.get_source_with_cache(&source2, CacheBehavior::Enabled)?;
		assert_eq!(result2.referenced().content, "first current"); // From cache!
		assert_eq!(result2.current().content, "second current"); // Fresh
		assert!(result2.diff().changed);

		// Cache should be unchanged (not updated on cache hit)
		let cached2 = cache.get::<TestReferenced>(&id)?.expect("Should still be cached");
		assert_eq!(cached2.content, "first current"); // Still original cached value

		Ok(())
	}
}
