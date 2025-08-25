use std::path::PathBuf;
use crate::{Source, SourceError, Comparison,  Referenced, Current, Diff};
use serde::{Serialize, Deserialize};
use crate::Id;

/// A cachable reference is serializable and deserializable
/// 
/// The reference only needs to read from the cache file.
pub trait CacheableReferenced: Referenced + Serialize + for<'de> Deserialize<'de> {

    fn from_cached_buffer(buffer: Vec<u8>) -> Result<Self, CacheError> {
        serde_json::from_slice(&buffer).map_err(CacheError::Deserialize)
    }

}

/// A cacheable current is serializable and deserializable
/// 
/// The current needs to be able to write to the cache file.
pub trait CacheableCurrent<R: CacheableReferenced, D: Diff> : Current<R, D> + Serialize + for<'de> Deserialize<'de> {

    fn to_cached_buffer(&self) -> Result<Vec<u8>, CacheError> {
        serde_json::to_vec(self).map_err(CacheError::Serialize)
    }
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

	#[error("Failed to delete cache file: {0}")]
	DeleteCacheFile(#[source] std::io::Error),

	#[error("Failed to serialize cacheable: {0}")]
	Serialize(#[source] serde_json::Error),

	#[error("Failed to deserialize cacheable: {0}")]
	Deserialize(#[source] serde_json::Error),

	#[error("Source error: {0}")]
	SourceError(#[source] SourceError),
}

pub struct Cache {
    builder: CacheBuilder,
}

#[derive(Debug, Clone)]
pub enum CacheBehavior {
    Enabled,
    Ignored
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
    let cache_buffer = std::fs::read_to_string(&cache_file).map_err(CacheError::ReadCacheFile)?;
    let cached_entry = R::from_cached_buffer(cache_buffer.as_bytes().to_vec())?;
    Ok(Some(cached_entry))
   }

   pub fn set<R: CacheableReferenced, C: CacheableCurrent<R, D>, D: Diff>(&self, id: &Id, value: &C) -> Result<(), CacheError> {
    let cache_file = self.cache_dir().join(id.as_str());
    let cache_buffer = value.to_cached_buffer()?;
    std::fs::write(&cache_file, cache_buffer).map_err(CacheError::WriteCacheFile)?;
    Ok(())
   }

   pub fn delete(&self, id: &Id) -> Result<(), CacheError> {
    let cache_file = self.cache_dir().join(id.as_str());
    std::fs::remove_file(&cache_file).map_err(CacheError::DeleteCacheFile)?;
    Ok(())
   }

   pub fn get_source_with_cache<S: Source<R, C, D>, R: CacheableReferenced, C: CacheableCurrent<R, D>, D: Diff>(&self, source: S, behavior: CacheBehavior) -> Result<Comparison<R, C, D>, CacheError> {
        let comparison = match behavior {
            CacheBehavior::Ignored => {
                let comparison = source.get().map_err(CacheError::SourceError)?;
                comparison
            }
            CacheBehavior::Enabled => {
                let referenced = match self.get::<R>(source.id())? {
                    Some(referenced) => referenced,
                    None => source.get_referenced().map_err(CacheError::SourceError)?,
                };
                let current = source.get_current().map_err(CacheError::SourceError)?;
                let diff = current.diff(&referenced).map_err(CacheError::SourceError)?;
                Comparison::new(referenced, current, diff)
            }
        };
        self.set(source.id(), comparison.current())?;
        Ok(comparison)  
   }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::Content;

    // Test implementations for CacheableReferenced and CacheableCurrent
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestReferenced {
        content: String,
    }

    impl Content for TestReferenced {}
    impl Referenced for TestReferenced {}
    impl CacheableReferenced for TestReferenced {}

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestCurrent {
        content: String,
    }

    impl Content for TestCurrent {}

    impl Current<TestReferenced, TestDiff> for TestCurrent {
        fn diff(&self, referenced: &TestReferenced) -> Result<TestDiff, SourceError> {
            Ok(TestDiff {
                changed: self.content != referenced.content,
            })
        }
    }

    impl CacheableCurrent<TestReferenced, TestDiff> for TestCurrent {}

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
        let _referenced = TestReferenced {
            content: "test content".to_string(),
        };

        // Test that cache is initially empty
        let result = cache.get::<TestReferenced>(&id)?;
        assert!(result.is_none());

        // Test set
        let current = TestCurrent {
            content: "test content".to_string(),
        };
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
        let referenced = TestReferenced {
            content: "test content".to_string(),
        };
        
        let serialized = serde_json::to_vec(&referenced)?;
        let deserialized = TestReferenced::from_cached_buffer(serialized)?;
        
        assert_eq!(referenced, deserialized);
        Ok(())
    }

    #[test]
    fn test_cacheable_current_serialization() -> Result<(), anyhow::Error> {
        let current = TestCurrent {
            content: "test content".to_string(),
        };
        
        let buffer = current.to_cached_buffer()?;
        let deserialized: TestCurrent = serde_json::from_slice(&buffer)?;
        
        assert_eq!(current, deserialized);
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

        let result = cache.get_source_with_cache(source, CacheBehavior::Ignored)?;
        
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

        let result = cache.get_source_with_cache(source, CacheBehavior::Enabled)?;
        
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

        let result = cache.get_source_with_cache(source, CacheBehavior::Enabled)?;
        
        // Should use cached referenced content
        assert_eq!(result.referenced().content, "cached content");
        // Should still fetch fresh current content
        assert_eq!(result.current().content, "current content");
        assert!(result.diff().changed);
        Ok(())
    }
}