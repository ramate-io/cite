/// Citation reporting level - determines how citation validation issues are reported
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitationLevel {
	/// Emit compilation errors for citation issues
	Error,
	/// Emit compilation warnings for citation issues
	Warn,
	/// Silently ignore citation issues
	Silent,
}

impl CitationLevel {
	/// Parse from string
	pub fn from_str(s: &str) -> Result<Self, String> {
		match s.to_lowercase().as_str() {
			"error" => Ok(CitationLevel::Error),
			"warn" => Ok(CitationLevel::Warn),
			"silent" => Ok(CitationLevel::Silent),
			_ => Err(format!("Invalid citation level: '{}'. Valid values: error, warn, silent", s)),
		}
	}

	/// Convert to string representation
	pub fn as_str(&self) -> &'static str {
		match self {
			CitationLevel::Error => "error",
			CitationLevel::Warn => "warn",
			CitationLevel::Silent => "silent",
		}
	}

	/// Check if this level should emit any output
	pub fn should_emit(&self) -> bool {
		!matches!(self, CitationLevel::Silent)
	}

	/// Check if this level should cause compilation failure
	pub fn should_fail_compilation(&self) -> bool {
		matches!(self, CitationLevel::Error)
	}
}

impl Default for CitationLevel {
	fn default() -> Self {
		CitationLevel::Warn
	}
}

impl std::fmt::Display for CitationLevel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_from_str() {
		assert_eq!(CitationLevel::from_str("error").unwrap(), CitationLevel::Error);
		assert_eq!(CitationLevel::from_str("WARN").unwrap(), CitationLevel::Warn);
		assert_eq!(CitationLevel::from_str("Silent").unwrap(), CitationLevel::Silent);
		assert!(CitationLevel::from_str("invalid").is_err());
	}

	#[test]
	fn test_should_emit() {
		assert!(CitationLevel::Error.should_emit());
		assert!(CitationLevel::Warn.should_emit());
		assert!(!CitationLevel::Silent.should_emit());
	}

	#[test]
	fn test_should_fail_compilation() {
		assert!(CitationLevel::Error.should_fail_compilation());
		assert!(!CitationLevel::Warn.should_fail_compilation());
		assert!(!CitationLevel::Silent.should_fail_compilation());
	}
}
