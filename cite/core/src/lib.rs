//! Cite Core - Fundamental abstractions for citation validation
//!
//! This crate provides the core traits, types, and utilities used by the cite
//! citation validation system. It's designed to be lightweight and focused,
//! containing only the essential abstractions needed for content validation.
//!
//! # Design Philosophy
//!
//! ## Separation of Concerns
//!
//! The core crate is deliberately separated from the procedural macro implementation:
//! - **No macro dependencies**: Can be used independently for runtime validation
//! - **Focused scope**: Only core abstractions and essential utilities
//! - **Lightweight**: Minimal dependencies and surface area
//!
//! ## Runtime and Compile-time Duality
//!
//! While the main cite system operates at compile time via procedural macros,
//! this crate provides utilities that work at both compile time and runtime:
//!
//! ```rust
//! use cite_core::{mock_source_same, mock_source_changed, Source};
//!
//! // Runtime usage - for testing and development
//! let source = mock_source_same("current content");
//! let comparison = source.get().unwrap();
//! assert!(comparison.is_same());
//!
//! // The same logic is used at compile time by the procedural macro
//! ```
//!
//! ## Standard Library Integration
//!
//! The crate uses standard library types throughout:
//! - `String` for content and error messages
//! - Standard error handling with `thiserror`
//! - Environment variable integration via `std::env`
//!
//! This design choice prioritizes simplicity and integration over `no_std` compatibility.
//!
//! # Key Abstractions
//!
//! ## Source Trait
//!
//! The fundamental abstraction for content sources:
//!
//! ```rust
//! use cite_core::{SourceError, Comparison, Referenced, Current, Diff};
//!
//! pub trait MySource {
//!     type Referenced: Referenced;
//!     type Current: Current<Self::Referenced, Self::Diff>;
//!     type Diff: Diff;
//!     
//!     fn get(&self) -> Result<Comparison<Self::Referenced, Self::Current, Self::Diff>, SourceError>;
//! }
//! ```
//!
//! ## Content Type System
//!
//! Content is modeled through three related traits:
//! - `Referenced`: The content as it was originally cited
//! - `Current`: The content as it exists now
//! - `Diff`: A representation of changes between referenced and current
//!
//! This separation enables rich content validation beyond simple string comparison.
//!
//! ## Behavior Configuration
//!
//! Citation behavior is controlled through environment-aware configuration:
//!
//! ```rust
//! use cite_core::{CitationBehavior, CitationLevel, mock_source_same, Source};
//!
//! // Load from features
//! let behavior = CitationBehavior::from_features();
//!
//! // Create a mock source and get comparison
//! let source = mock_source_same("example content");
//! let comparison = source.get().unwrap();
//!
//! // Override locally
//! let result = comparison.validate(&behavior, Some(CitationLevel::Warn));
//! ```
//!
//! # Mock Implementation
//!
//! The `MockSource` provides a complete implementation for testing and development:
//!
//! ```rust
//! use cite_core::{mock_source_same, mock_source_changed};
//!
//! // Content that should remain unchanged
//! let unchanged = mock_source_same("stable API");
//!
//! // Content that has changed
//! let changed = mock_source_changed("old version", "new version");
//! ```
//!
//! The mock implementation serves multiple purposes:
//! - **Testing**: Enables comprehensive testing without external dependencies
//! - **Development**: Prototyping citation behavior before connecting real sources
//! - **Documentation**: Clear examples of how the Source trait should behave
//!
//! # Future Extensions
//!
//! The trait system is designed to support additional source types:
//! - HTTP sources for web content validation
//! - File sources for local content validation
//! - Git sources for repository content validation
//! - Database sources for schema and data validation
//!
//! New source types integrate seamlessly with the existing validation and behavior system.

pub mod behavior;
pub mod id;
pub mod mock;

pub use behavior::{CitationAnnotation, CitationBehavior, CitationGlobal, CitationLevel};
pub use id::Id;
pub use mock::{mock_source_changed, mock_source_same, MockSource};

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
pub trait Content {}

/// [Referenced] marks the [Content] type that was originally referenced by the [Source].
pub trait Referenced: Content {}

/// [Current] marks the [Content] type that is currently available via the [Source].
///
/// It should be able to able to [Diff] against a [Referenced] type.
pub trait Current<R: Referenced, D: Diff>: Content {
	fn diff(&self, other: &R) -> Result<D, SourceError>;
}

/// [Source] is a trait that allows for the creation of a [Content] type.
pub trait Source<R: Referenced, C: Current<R, D>, D: Diff> {
	fn id(&self) -> &Id;

	fn name(&self) -> &str {
		self.id().as_str()
	}

	fn link(&self) -> &str {
		self.id().as_str()
	}

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

impl<R, C, D> Comparison<R, C, D>
where
	R: Referenced,
	C: Current<R, D>,
	D: Diff,
{
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
	pub fn validate(
		&self,
		behavior: &CitationBehavior,
		local_level: Option<CitationLevel>,
	) -> CitationValidationResult {
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
			CitationValidationResult::Invalid { should_fail_compilation, .. } => {
				*should_fail_compilation
			}
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
