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

   pub fn set<R: CacheableReferenced, C: CacheableCurrent<R, D>, D: Diff>(&self, id: &Id, value: C) -> Result<(), CacheError> {
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
    match behavior {
        CacheBehavior::Ignored => {
            let comparison = source.get().map_err(CacheError::SourceError)?;
            Ok(comparison)
        }
        CacheBehavior::Enabled => {
        let referenced = match self.get::<R>(source.id())? {
            Some(referenced) => referenced,
            None => source.get_referenced().map_err(CacheError::SourceError)?,
        };
        let current = source.get_current().map_err(CacheError::SourceError)?;
        let diff = current.diff(&referenced).map_err(CacheError::SourceError)?;
        Ok(Comparison::new(referenced, current, diff))
        }
    }   
   }
}