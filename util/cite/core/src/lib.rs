pub mod mock;
pub mod behavior;
pub mod cache;
pub mod id;
pub mod hypertext;

pub use id::Id;
pub use behavior::{CitationBehavior, CitationLevel, CitationAnnotation, CitationGlobal};
pub use mock::{MockSource, mock_source_same, mock_source_changed};
pub use cache::{CacheableReferenced, CacheableCurrent, CacheBuilder, Cache, CacheError, CacheBuilderError, CacheBehavior};
pub use hypertext::{HypertextMatch, MatchExpression, SourceUrl, ReferencedHypertext, CurrentHypertext, HypertextDiff};

/// Errors thrown by the [Source].
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
	#[error("Source internal error: {0}")]
	Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
	
	#[error("Network error: {0}")]
	Network(String),
	
	#[error("Cache error: {0}")]
	Cache(String),
	
	#[error("Content parsing error: {0}")]
	ContentParsing(String),
	
	#[error("External dependency error: {0}")]
	ExternalDependency(String),
}

/// [Diff] is a trait that contains information as to the diff between two [Content] types.
/// 
/// TODO: we need to standardize a diff output format, s.t., we can add a method to the [Source] trait.
pub trait Diff {
    
    fn is_empty(&self) -> bool;

}


/// [Content] is a marker trait.
/// 
/// TODO: we should constrain this to have some kind of formatter.
pub trait Content {

}

/// [Referenced] marks the [Content] type that was originally referenced by the [Source].
pub trait Referenced: Content {
    
}

/// [Current] marks the [Content] type that is currently available via the [Source].
/// 
/// It should be able to able to [Diff] against a [Referenced] type.
pub trait Current<R: Referenced, D: Diff>: Content {
    fn diff(&self, other: &R) -> Result<D, SourceError>;
}

/// [Source] is a trait that allows for the creation of a [Content] type.
pub trait Source<R: Referenced, C: Current<R, D>, D: Diff> {

    fn id(&self) -> &Id;

    fn get_referenced(&self) -> Result<R, SourceError>;

    fn get_current(&self) -> Result<C, SourceError>;

    fn get(&self) -> Result<Comparison<R, C, D>, SourceError> {
        let referenced = self.get_referenced()?;
        let current = self.get_current()?;
        let diff = current.diff(&referenced)?;
        Ok(Comparison::new(referenced, current, diff))
    }
}

/// [Comparison] is the result of getting a source. 
pub struct Comparison<R: Referenced, C: Current<R, D>, D: Diff> {
    pub referenced: R,
    pub current: C,
    pub diff: D,
}

impl <R, C, D> Comparison<R, C, D> where R: Referenced, C: Current<R, D>, D: Diff {
    pub fn new(referenced: R, current: C, diff: D) -> Self {
        Self { referenced, current, diff }
    }

    pub fn referenced(&self) -> &R {
        &self.referenced
    }

    pub fn current(&self) -> &C {
        &self.current
    }

    pub fn diff(&self) -> &D {
        &self.diff
    }

    pub fn is_same(&self) -> bool {
        self.diff.is_empty()
    }
    
    /// Validate this comparison against behavior configuration
    pub fn validate(&self, behavior: &CitationBehavior, local_level: Option<CitationLevel>) -> CitationValidationResult {
        if self.is_same() {
            CitationValidationResult::Valid
        } else {
            let effective_level = behavior.effective_level(local_level);
            CitationValidationResult::Invalid {
                level: effective_level,
                should_fail_compilation: behavior.should_fail_compilation(local_level),
                should_report: behavior.should_report(local_level),
            }
        }
    }
}

/// Result of citation validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationValidationResult {
    /// Citation is valid (content matches)
    Valid,
    /// Citation is invalid (content has changed)
    Invalid {
        /// Effective reporting level
        level: CitationLevel,
        /// Whether this should fail compilation
        should_fail_compilation: bool,
        /// Whether this should be reported
        should_report: bool,
    },
}

impl CitationValidationResult {
    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        matches!(self, CitationValidationResult::Valid)
    }
    
    /// Check if this result should fail compilation
    pub fn should_fail_compilation(&self) -> bool {
        match self {
            CitationValidationResult::Valid => false,
            CitationValidationResult::Invalid { should_fail_compilation, .. } => *should_fail_compilation,
        }
    }
    
    /// Check if this result should be reported
    pub fn should_report(&self) -> bool {
        match self {
            CitationValidationResult::Valid => false,
            CitationValidationResult::Invalid { should_report, .. } => *should_report,
        }
    }
    
    /// Get the reporting level if invalid
    pub fn level(&self) -> Option<CitationLevel> {
        match self {
            CitationValidationResult::Valid => None,
            CitationValidationResult::Invalid { level, .. } => Some(*level),
        }
    }
}