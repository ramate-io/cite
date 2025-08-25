//! Core trait definitions for citation validation
//! 
//! These traits are lightweight and no_std compatible, defining the interface
//! for citation sources without heavy implementation dependencies.



use crate::{SourceError, Id};

/// Content marker trait for citation data
/// 
/// This trait is implemented by both referenced and current content types.
/// It's intentionally lightweight with no required methods.
pub trait Content {}

/// Referenced content that was cited at commit time
/// 
/// This represents the "known good" state that was referenced when the citation
/// was originally made.
pub trait Referenced: Content {}

/// Trait for generating diffs between content versions
/// 
/// Implementations should provide meaningful diff information that can be
/// displayed to developers when content changes.
pub trait Diff {
    /// Returns true if the diff represents no changes
    fn is_empty(&self) -> bool;
}

/// Current content fetched from the source
/// 
/// This represents the live state of the cited content. The `diff` method
/// compares against the referenced content to detect changes.
pub trait Current<R: Referenced, D: Diff>: Content {
    /// Generate a diff between current and referenced content
    fn diff(&self, referenced: &R) -> Result<D, SourceError>;
}

/// A comparison between referenced and current content
/// 
/// This struct holds the three components of a citation validation:
/// - Referenced: The original cited content
/// - Current: The current state of the content  
/// - Diff: The difference between them
#[derive(Debug, Clone)]
pub struct Comparison<R: Referenced, C: Current<R, D>, D: Diff> {
    referenced: R,
    current: C, 
    diff: D,
}

impl<R: Referenced, C: Current<R, D>, D: Diff> Comparison<R, C, D> {
    /// Create a new comparison
    pub fn new(referenced: R, current: C, diff: D) -> Self {
        Self { referenced, current, diff }
    }
    
    /// Get the referenced content
    pub fn referenced(&self) -> &R {
        &self.referenced
    }
    
    /// Get the current content
    pub fn current(&self) -> &C {
        &self.current
    }
    
    /// Get the diff
    pub fn diff(&self) -> &D {
        &self.diff
    }
    
    /// Check if the content is the same (no changes)
    pub fn is_same(&self) -> bool {
        self.diff.is_empty()
    }
}

/// A source that can be validated for citations
/// 
/// Sources provide both referenced (historical) and current content,
/// along with the ability to generate a full comparison.
pub trait Source<R: Referenced, C: Current<R, D>, D: Diff> {
    /// Get the unique identifier for this source
    fn id(&self) -> &Id;
    
    /// Get only the referenced (historical) content
    fn get_referenced(&self) -> Result<R, SourceError>;
    
    /// Get only the current content
    fn get_current(&self) -> Result<C, SourceError>;
    
    /// Get a complete comparison (referenced, current, and diff)
    fn get(&self) -> Result<Comparison<R, C, D>, SourceError> {
        let referenced = self.get_referenced()?;
        let current = self.get_current()?;
        let diff = current.diff(&referenced)?;
        Ok(Comparison::new(referenced, current, diff))
    }
}

/// A referenced content type that can be cached
/// 
/// This trait allows the citation system to store referenced content
/// for later comparison, avoiding repeated fetches.
pub trait CacheableReferenced: Referenced + Sized {
    /// Deserialize from cached buffer
    fn from_cached_buffer(buffer: &[u8]) -> Result<Self, CacheError>;
}

/// A current content type that can be cached
/// 
/// This trait allows current content to be serialized for caching.
/// Note that current content is typically cached as referenced content
/// for future comparisons.
pub trait CacheableCurrent<R: CacheableReferenced, D: Diff>: Current<R, D> + Sized {
    /// Serialize to cached buffer
    fn to_cached_buffer(&self) -> Result<impl AsRef<[u8]>, CacheError>;
}

/// Errors that can occur during cache operations
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum CacheError {
    /// Failed to read from cache
    #[cfg_attr(feature = "std", error("Cache read failed: {0}"))]
    ReadFailure(&'static str),
    
    /// Failed to write to cache
    #[cfg_attr(feature = "std", error("Cache write failed: {0}"))]
    WriteFailure(&'static str),
    
    /// Failed to serialize content
    #[cfg_attr(feature = "std", error("Serialization failed: {0}"))]
    SerializationFailure(&'static str),
    
    /// Failed to deserialize content
    #[cfg_attr(feature = "std", error("Deserialization failed: {0}"))]
    DeserializationFailure(&'static str),
}

/// Cache behavior configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheBehavior {
    /// Use cache when available, populate on miss
    Enabled,
    /// Ignore cache entirely, always fetch fresh
    Ignored,
}
