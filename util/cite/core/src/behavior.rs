pub mod level;
pub mod annotation;
pub mod global;

pub use level::CitationLevel;
pub use annotation::CitationAnnotation;
pub use global::CitationGlobal;

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
    pub fn new(level: CitationLevel, annotation: CitationAnnotation, global: CitationGlobal) -> Self {
        Self { level, annotation, global }
    }
    

    
    /// Resolve the effective citation level, considering local overrides
    pub fn effective_level(&self, local_level: Option<CitationLevel>) -> CitationLevel {
        match (self.global.allows_local_overrides(), local_level) {
            (true, Some(local)) => local,  // Local override allowed and provided
            _ => self.level,               // Use global level
        }
    }
    
    /// Resolve the effective annotation requirement, considering local overrides
    pub fn effective_annotation(&self, local_annotation: Option<CitationAnnotation>) -> CitationAnnotation {
        match (self.global.allows_local_overrides(), local_annotation) {
            (true, Some(local)) => local,  // Local override allowed and provided
            _ => self.annotation,          // Use global annotation
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
        assert_eq!(
            behavior.effective_level(Some(CitationLevel::Error)),
            CitationLevel::Error
        );
        
        // Global level used when no local override
        assert_eq!(
            behavior.effective_level(None),
            CitationLevel::Warn
        );
    }
    
    #[test]
    fn test_effective_level_with_strict_global() {
        let behavior = CitationBehavior::new(
            CitationLevel::Warn,
            CitationAnnotation::Any,
            CitationGlobal::Strict,
        );
        
        // Global level always used in strict mode
        assert_eq!(
            behavior.effective_level(Some(CitationLevel::Error)),
            CitationLevel::Warn
        );
        
        assert_eq!(
            behavior.effective_level(None),
            CitationLevel::Warn
        );
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
}