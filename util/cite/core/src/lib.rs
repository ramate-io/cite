#![no_std]

pub mod behavior;
pub mod id;
pub mod traits;
pub mod mock;

// Re-export core types that should be available everywhere
pub use id::Id;
pub use behavior::{CitationBehavior, CitationLevel, CitationAnnotation, CitationGlobal};
pub use traits::*;
pub use mock::{MockSource, mock_source_same, mock_source_changed};

/// Errors thrown by citation sources
/// 
/// This error type is designed to be lightweight and no_std compatible.
#[derive(Debug)]
pub enum SourceError {
    /// Internal source error
    Internal(&'static str),
    
    /// Network-related error  
    Network(&'static str),
    
    /// Cache-related error
    Cache(&'static str),
    
    /// Content parsing error
    ContentParsing(&'static str),
    
    /// External dependency error
    ExternalDependency(&'static str),
}

// Core traits are now defined in traits.rs and re-exported above

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