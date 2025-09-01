pub mod annotation;
pub mod global;
pub mod level;

pub use annotation::CitationAnnotation;
pub use global::CitationGlobal;
pub use level::CitationLevel;

/// Complete citation behavior configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationBehavior {
	/// How to report citation validation issues
	pub level: CitationLevel,
	/// When citations are required/allowed
	pub annotation: CitationAnnotation,
	/// Whether local overrides are allowed
	pub global: CitationGlobal,
}

impl CitationBehavior {
	/// Create a new behavior configuration
	pub fn new(
		level: CitationLevel,
		annotation: CitationAnnotation,
		global: CitationGlobal,
	) -> Self {
		Self { level, annotation, global }
	}

	/// Load configuration from feature flags
	pub fn from_features() -> Self {
		// Check feature flags for citation level
		// Default is Error (no feature flag needed)
		#[cfg(feature = "warn")]
		let level = CitationLevel::Warn;
		#[cfg(feature = "silent")]
		let level = CitationLevel::Silent;
		#[cfg(not(any(feature = "warn", feature = "silent")))]
		let level = CitationLevel::Error; // Default to error

		// Check feature flags for annotation requirement
		// Default is Footnote (no feature flag needed)
		#[cfg(feature = "annotationless")]
		let annotation = CitationAnnotation::Any;
		#[cfg(not(feature = "annotationless"))]
		let annotation = CitationAnnotation::Footnote; // Default to footnote

		// Check feature flags for global behavior
		// Default is Strict (no feature flag needed)
		#[cfg(feature = "lenient")]
		let global = CitationGlobal::Lenient;
		#[cfg(not(feature = "lenient"))]
		let global = CitationGlobal::Strict; // Default to strict

		Self { level, annotation, global }
	}

	/// Resolve the effective citation level, considering local overrides
	pub fn effective_level(&self, local_level: Option<CitationLevel>) -> CitationLevel {
		match (self.global.allows_local_overrides(), local_level) {
			(true, Some(local)) => local, // Local override allowed and provided
			_ => self.level,              // Use global level
		}
	}

	/// Resolve the effective annotation requirement, considering local overrides
	pub fn effective_annotation(
		&self,
		local_annotation: Option<CitationAnnotation>,
	) -> CitationAnnotation {
		match (self.global.allows_local_overrides(), local_annotation) {
			(true, Some(local)) => local, // Local override allowed and provided
			_ => self.annotation,         // Use global annotation
		}
	}

	/// Checks if an annotation is required
	pub fn requires_effective_annotation(
		&self,
		local_annotation: Option<CitationAnnotation>,
	) -> bool {
		match (self.global.allows_local_overrides(), local_annotation, self.annotation) {
			(true, Some(CitationAnnotation::Any), _) => false, // Local override allowed and provided
			(false, _, CitationAnnotation::Any) => false,      // Global annotation is Any
			_ => true,                                         // Use global annotation
		}
	}

	/// Check if a citation validation issue should be reported
	pub fn should_report(&self, local_level: Option<CitationLevel>) -> bool {
		self.effective_level(local_level).should_emit()
	}

	/// Check if a citation validation issue should fail compilation
	pub fn should_fail_compilation(&self, local_level: Option<CitationLevel>) -> bool {
		self.effective_level(local_level).should_fail_compilation()
	}
}

impl Default for CitationBehavior {
	fn default() -> Self {
		Self {
			level: CitationLevel::default(),
			annotation: CitationAnnotation::default(),
			global: CitationGlobal::default(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_effective_level_with_lenient_global() {
		let behavior = CitationBehavior::new(
			CitationLevel::Warn,
			CitationAnnotation::Any,
			CitationGlobal::Lenient,
		);

		// Local override should be used in lenient mode
		assert_eq!(behavior.effective_level(Some(CitationLevel::Error)), CitationLevel::Error);

		// Global level used when no local override
		assert_eq!(behavior.effective_level(None), CitationLevel::Warn);
	}

	#[test]
	fn test_effective_level_with_strict_global() {
		let behavior = CitationBehavior::new(
			CitationLevel::Warn,
			CitationAnnotation::Any,
			CitationGlobal::Strict,
		);

		// Global level always used in strict mode
		assert_eq!(behavior.effective_level(Some(CitationLevel::Error)), CitationLevel::Warn);

		assert_eq!(behavior.effective_level(None), CitationLevel::Warn);
	}

	#[test]
	fn test_should_report() {
		let behavior = CitationBehavior::new(
			CitationLevel::Warn,
			CitationAnnotation::Any,
			CitationGlobal::Lenient,
		);

		assert!(behavior.should_report(Some(CitationLevel::Error)));
		assert!(behavior.should_report(Some(CitationLevel::Warn)));
		assert!(!behavior.should_report(Some(CitationLevel::Silent)));
		assert!(behavior.should_report(None)); // Uses global Warn
	}

	#[test]
	fn test_should_fail_compilation() {
		let behavior = CitationBehavior::new(
			CitationLevel::Warn,
			CitationAnnotation::Any,
			CitationGlobal::Lenient,
		);

		assert!(behavior.should_fail_compilation(Some(CitationLevel::Error)));
		assert!(!behavior.should_fail_compilation(Some(CitationLevel::Warn)));
		assert!(!behavior.should_fail_compilation(Some(CitationLevel::Silent)));
		assert!(!behavior.should_fail_compilation(None)); // Uses global Warn
	}

	#[test]
	fn test_requires_effective_annotation() {
		let behavior = CitationBehavior::new(
			CitationLevel::Warn,
			CitationAnnotation::Any,
			CitationGlobal::Lenient,
		);

		assert!(!behavior.requires_effective_annotation(Some(CitationAnnotation::Any)));
		assert!(behavior.requires_effective_annotation(Some(CitationAnnotation::Footnote)));
		assert!(behavior.requires_effective_annotation(None));
	}
}
