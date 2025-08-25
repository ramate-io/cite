/// Citation annotation requirements - determines when citations are required
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitationAnnotation {
    /// Require citations only on items with doc comments/footnotes
    Footnote,
    /// Allow citations on any code item regardless of documentation
    Any,
}

impl CitationAnnotation {
    /// Parse from environment variable or string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "footnote" => Ok(CitationAnnotation::Footnote),
            "any" => Ok(CitationAnnotation::Any),
            _ => Err(format!("Invalid citation annotation: '{}'. Valid values: footnote, any", s)),
        }
    }
    
    /// Get from CITE_ANNOTATION environment variable or return default
    pub fn from_env() -> Self {
        std::env::var("CITE_ANNOTATION")
            .ok()
            .and_then(|s| Self::from_str(&s).ok())
            .unwrap_or(CitationAnnotation::Any) // Default to any
    }
    
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            CitationAnnotation::Footnote => "footnote",
            CitationAnnotation::Any => "any",
        }
    }
    
    /// Check if citations are allowed on items without documentation
    pub fn allows_undocumented(&self) -> bool {
        matches!(self, CitationAnnotation::Any)
    }
}

impl Default for CitationAnnotation {
    fn default() -> Self {
        CitationAnnotation::Any
    }
}

impl std::fmt::Display for CitationAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(CitationAnnotation::from_str("footnote").unwrap(), CitationAnnotation::Footnote);
        assert_eq!(CitationAnnotation::from_str("ANY").unwrap(), CitationAnnotation::Any);
        assert!(CitationAnnotation::from_str("invalid").is_err());
    }

    #[test]
    fn test_allows_undocumented() {
        assert!(!CitationAnnotation::Footnote.allows_undocumented());
        assert!(CitationAnnotation::Any.allows_undocumented());
    }
}
