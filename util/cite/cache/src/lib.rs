use std::path::PathBuf;
use cite_util_core::{
    Source, SourceError, Comparison, Referenced, Current, Diff, 
    CacheableReferenced, CacheableCurrent, CacheError, CacheBehavior, Id
};
use serde::{Serialize, Deserialize};

/// A cachable reference is serializable and deserializable
/// 
/// The reference only needs to read from the cache file.
impl<T> CacheableReferenced for T 
where 
    T: Referenced + Serialize + for<'de> Deserialize<'de>
{
    fn from_cached_buffer(buffer: &[u8]) -> Result<Self, CacheError> {
        serde_json::from_slice(buffer).map_err(|e| CacheError::DeserializationFailure("serde_json failed"))
    }
}

/// A cacheable current is serializable and deserializable
/// 
/// The current needs to be able to write to the cache file.
impl<R, D, T> CacheableCurrent<R, D> for T 
where 
    T: Current<R, D> + Serialize + for<'de> Deserialize<'de>,
    R: CacheableReferenced,
    D: Diff,
{
    fn to_cached_buffer(&self) -> Result<impl AsRef<[u8]>, CacheError> {
        serde_json::to_vec(self).map_err(|e| CacheError::SerializationFailure("serde_json failed"))
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

pub struct Cache {
    builder: CacheBuilder,
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
    let cache_buffer = std::fs::read(&cache_file).map_err(|_| CacheError::ReadFailure("file read failed"))?;
    let cached_entry = R::from_cached_buffer(&cache_buffer)?;
    Ok(Some(cached_entry))
   }

   pub fn set<R: CacheableReferenced, C: CacheableCurrent<R, D>, D: Diff>(&self, id: &Id, value: &C) -> Result<(), CacheError> {
    let cache_file = self.cache_dir().join(id.as_str());
    let cache_buffer = value.to_cached_buffer()?;
    std::fs::write(&cache_file, cache_buffer.as_ref()).map_err(|_| CacheError::WriteFailure("file write failed"))?;
    Ok(())
   }

   pub fn delete(&self, id: &Id) -> Result<(), CacheError> {
    let cache_file = self.cache_dir().join(id.as_str());
    std::fs::remove_file(&cache_file).map_err(|_| CacheError::WriteFailure("file delete failed"))?;
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
   pub fn get_source_with_cache<S: Source<R, C, D>, R: CacheableReferenced, C: CacheableCurrent<R, D>, D: Diff>(&self, source: S, behavior: CacheBehavior) -> Result<Comparison<R, C, D>, CacheError> {
    match behavior {
            CacheBehavior::Ignored => {
                let comparison = source.get().map_err(|e| CacheError::ReadFailure("source get failed"))?;
                self.set(source.id(), comparison.current())?;
                Ok(comparison)
            }
            CacheBehavior::Enabled => {
                let (referenced, current) = match self.get::<R>(source.id())? {
                    Some(referenced) => (referenced, source.get_current().map_err(|e| CacheError::ReadFailure("source get_current failed"))?),
                    None => {
                        let referenced = source.get_referenced().map_err(|e| CacheError::ReadFailure("source get_referenced failed"))?;
                        let current = source.get_current().map_err(|e| CacheError::ReadFailure("source get_current failed"))?;
                        self.set(source.id(), &current)?;
                        (referenced, current)
                    }
                };
                let diff = current.diff(&referenced).map_err(|e| CacheError::ReadFailure("diff failed"))?;
                let comparison = Comparison::new(referenced, current, diff);
                Ok(comparison)
            }
        }
        
   }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use cite_util_core::Content;
    use serde::{Serialize, Deserialize};
    use anyhow::Result;

    // Test implementations for CacheableReferenced and CacheableCurrent
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestReferenced {
        content: String,
    }

    impl Content for TestReferenced {}
    impl Referenced for TestReferenced {}

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestCurrent {
        content: String,
    }

    impl Content for TestCurrent {}

    #[derive(Debug, Clone, PartialEq)]
    struct TestDiff {
        changed: bool,
    }

    impl Diff for TestDiff {
        fn is_empty(&self) -> bool {
            !self.changed
        }
    }

    impl Current<TestReferenced, TestDiff> for TestCurrent {
        fn diff(&self, referenced: &TestReferenced) -> Result<TestDiff, SourceError> {
            Ok(TestDiff {
                changed: self.content != referenced.content,
            })
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
    fn test_cache_functionality() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
        let cache = builder.build()?;
        
        let id = Id::new("test-key");
        let current = TestCurrent {
            content: "test content".to_string(),
        };
        
        // Test that cache is initially empty
        let result = cache.get::<TestReferenced>(&id)?;
        assert!(result.is_none());

        // Test set
        cache.set(&id, &current)?;

        // Test get after set
        let result = cache.get::<TestReferenced>(&id)?;
        assert!(result.is_some());
        
        Ok(())
    }

    #[test]
    fn test_cache_with_source() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), PathBuf::from("cache"));
        let cache = builder.build()?;
        
        let source = TestSource {
            id: Id::new("test-source"),
            referenced: TestReferenced { content: "ref content".to_string() },
            current: TestCurrent { content: "current content".to_string() },
        };

        let result = cache.get_source_with_cache(source, CacheBehavior::Enabled)?;
        
        assert_eq!(result.referenced().content, "ref content");
        assert_eq!(result.current().content, "current content");
        assert!(result.diff().changed);
        
        Ok(())
    }
}
