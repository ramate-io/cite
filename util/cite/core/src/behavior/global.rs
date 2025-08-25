/// Global citation enforcement mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitationGlobal {
    /// Strict mode - local overrides are not allowed, global settings take precedence
    Strict,
    /// Lenient mode - allow local citation attributes to override global settings
    Lenient,
}

impl CitationGlobal {
    /// Parse from environment variable or string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "strict" => Ok(CitationGlobal::Strict),
            "lenient" => Ok(CitationGlobal::Lenient),
            _ => Err(format!("Invalid citation global mode: '{}'. Valid values: strict, lenient", s)),
        }
    }
    
    /// Get from CITE_GLOBAL environment variable or return default
    pub fn from_env() -> Self {
        std::env::var("CITE_GLOBAL")
            .ok()
            .and_then(|s| Self::from_str(&s).ok())
            .unwrap_or(CitationGlobal::Lenient) // Default to lenient
    }
    
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            CitationGlobal::Strict => "strict",
            CitationGlobal::Lenient => "lenient",
        }
    }
    
    /// Check if local overrides are allowed
    pub fn allows_local_overrides(&self) -> bool {
        matches!(self, CitationGlobal::Lenient)
    }
}

impl Default for CitationGlobal {
    fn default() -> Self {
        CitationGlobal::Lenient
    }
}

impl std::fmt::Display for CitationGlobal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(CitationGlobal::from_str("strict").unwrap(), CitationGlobal::Strict);
        assert_eq!(CitationGlobal::from_str("LENIENT").unwrap(), CitationGlobal::Lenient);
        assert!(CitationGlobal::from_str("invalid").is_err());
    }

    #[test]
    fn test_allows_local_overrides() {
        assert!(!CitationGlobal::Strict.allows_local_overrides());
        assert!(CitationGlobal::Lenient.allows_local_overrides());
    }
}
