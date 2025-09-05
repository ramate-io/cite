use crate::{CurrentGitContent, GitDiff, GitSource, ReferencedGitContent};
use cite_core::ui::{AboveDocAttr, SourceUi, SourceUiError};
use serde_json::{Map, Value};
use std::collections::HashMap;

impl SourceUi<ReferencedGitContent, CurrentGitContent, GitDiff> for GitSource {
	fn from_kwarg_json(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// First, try direct deserialization from the kwargs
		if let Ok(source) = Self::try_direct_deserialization(kwargs) {
			return Ok(source);
		}

		// If direct deserialization fails, try manual parameter extraction
		Self::try_manual_extraction(kwargs)
	}

	fn to_standard_json(&self) -> Result<Map<String, Value>, SourceUiError> {
		// Use direct serialization to JSON, then convert to Map
		let json_value = serde_json::to_value(self).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to serialize GitSource: {}", e))
		})?;

		let mut map = json_value
			.as_object()
			.ok_or_else(|| {
				SourceUiError::Serialization(
					"GitSource serialization did not produce an object".to_string(),
				)
			})?
			.clone();

		// Add the src field for consistency
		map.insert("src".to_string(), Value::String("git".to_string()));

		Ok(map)
	}

	fn to_above_doc_attr(&self) -> Result<AboveDocAttr, SourceUiError> {
		let json_map = self.to_standard_json()?;
		let json_content = serde_json::to_string_pretty(&json_map).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to serialize to JSON: {}", e))
		})?;

		Ok(AboveDocAttr::new(json_content, "git".to_string()))
	}

	fn is_valid_attr_key(attr_key: &str) -> bool {
		match attr_key {
			// Direct serde fields
			"id" | "remote" | "path_pattern" | "referenced_revision" | "current_revision" | "name" | "formatted_url" | "repository_builder" |
			// Legacy ergonomic fields
			"ref_rev" | "cur_rev" | "path" |
			// Citation-level fields
			"src" | "reason" | "level" | "annotation" => true,
			_ => false,
		}
	}
}

impl GitSource {
	/// Try to deserialize GitSource directly from kwargs using serde
	fn try_direct_deserialization(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// Convert HashMap to JSON and try to deserialize
		let json_value = serde_json::to_value(kwargs).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to convert kwargs to JSON: {}", e))
		})?;

		// Try to deserialize as GitSource directly
		let source: GitSource = serde_json::from_value(json_value).map_err(|e| {
			SourceUiError::Serialization(format!("Direct deserialization failed: {}", e))
		})?;

		Ok(source)
	}

	/// Try manual parameter extraction for backward compatibility
	fn try_manual_extraction(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// Extract required parameters
		let remote = kwargs
			.get("remote")
			.and_then(|v| v.as_str())
			.ok_or_else(|| SourceUiError::MissingParameter("remote".to_string()))?;

		let ref_rev = kwargs
			.get("ref_rev")
			.or_else(|| kwargs.get("referenced_revision"))
			.and_then(|v| v.as_str())
			.ok_or_else(|| {
				SourceUiError::MissingParameter("ref_rev or referenced_revision".to_string())
			})?;

		let cur_rev = kwargs
			.get("cur_rev")
			.or_else(|| kwargs.get("current_revision"))
			.and_then(|v| v.as_str())
			.ok_or_else(|| {
				SourceUiError::MissingParameter("cur_rev or current_revision".to_string())
			})?;

		let path = kwargs
			.get("path")
			.and_then(|v| v.as_str())
			.ok_or_else(|| SourceUiError::MissingParameter("path".to_string()))?;

		// Extract optional name parameter
		let name = kwargs.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());

		// Validate URL format
		if !is_valid_git_url(remote) {
			return Err(SourceUiError::InvalidParameter(format!(
				"Invalid Git remote URL format: {}",
				remote
			)));
		}

		// Create the GitSource
		GitSource::try_new(remote, path, ref_rev, cur_rev, name)
			.map_err(|e| SourceUiError::Internal(e.into()))
	}
}

/// Basic Git URL validation for parse-time checking
fn is_valid_git_url(url: &str) -> bool {
	url.starts_with("https://") || url.starts_with("http://") || url.starts_with("git@")
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_from_kwarg_json_basic() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo.git"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		kwargs.insert("cur_rev".to_string(), json!("def456"));
		kwargs.insert("path".to_string(), json!("src/main.rs"));
		kwargs.insert("name".to_string(), json!("test-name"));

		let git_source = GitSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(git_source.remote, "https://github.com/user/repo.git");
		assert_eq!(git_source.referenced_revision, "abc123");
		assert_eq!(git_source.current_revision, "def456");
		assert_eq!(git_source.path_pattern.to_string(), "src/main.rs");
		assert_eq!(git_source.name, "test-name");
	}

	#[test]
	fn test_from_kwarg_json_legacy_field_names() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo.git"));
		kwargs.insert("referenced_revision".to_string(), json!("abc123"));
		kwargs.insert("current_revision".to_string(), json!("def456"));
		kwargs.insert("path".to_string(), json!("src/main.rs"));

		let git_source = GitSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(git_source.referenced_revision, "abc123");
		assert_eq!(git_source.current_revision, "def456");
	}

	#[test]
	fn test_from_kwarg_json_without_name() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo.git"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		kwargs.insert("cur_rev".to_string(), json!("def456"));
		kwargs.insert("path".to_string(), json!("src/main.rs"));

		let git_source = GitSource::from_kwarg_json(&kwargs).unwrap();
		// Name should be generated automatically
		assert!(!git_source.name.is_empty());
	}

	#[test]
	fn test_from_kwarg_json_missing_remote() {
		let mut kwargs = HashMap::new();
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		kwargs.insert("cur_rev".to_string(), json!("def456"));
		kwargs.insert("path".to_string(), json!("src/main.rs"));

		let result = GitSource::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), SourceUiError::MissingParameter(_)));
	}

	#[test]
	fn test_from_kwarg_json_missing_ref_rev() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo.git"));
		kwargs.insert("cur_rev".to_string(), json!("def456"));
		kwargs.insert("path".to_string(), json!("src/main.rs"));

		let result = GitSource::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), SourceUiError::MissingParameter(_)));
	}

	#[test]
	fn test_from_kwarg_json_missing_cur_rev() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo.git"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		kwargs.insert("path".to_string(), json!("src/main.rs"));

		let result = GitSource::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), SourceUiError::MissingParameter(_)));
	}

	#[test]
	fn test_from_kwarg_json_missing_path() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo.git"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		kwargs.insert("cur_rev".to_string(), json!("def456"));

		let result = GitSource::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), SourceUiError::MissingParameter(_)));
	}

	#[test]
	fn test_from_kwarg_json_invalid_remote_url() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("invalid-url"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		kwargs.insert("cur_rev".to_string(), json!("def456"));
		kwargs.insert("path".to_string(), json!("src/main.rs"));

		let result = GitSource::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), SourceUiError::InvalidParameter(_)));
	}

	#[test]
	fn test_from_kwarg_json_valid_url_formats() {
		let test_cases = vec![
			"https://github.com/user/repo.git",
			"http://github.com/user/repo.git",
			"git@github.com:user/repo.git",
		];

		for remote in test_cases {
			let mut kwargs = HashMap::new();
			kwargs.insert("remote".to_string(), json!(remote));
			kwargs.insert("ref_rev".to_string(), json!("abc123"));
			kwargs.insert("cur_rev".to_string(), json!("def456"));
			kwargs.insert("path".to_string(), json!("src/main.rs"));

			let result = GitSource::from_kwarg_json(&kwargs);
			assert!(result.is_ok(), "Failed for remote: {}", remote);
		}
	}

	#[test]
	fn test_to_standard_json_basic() {
		let git_source = GitSource::try_new(
			"https://github.com/user/repo.git",
			"src/main.rs",
			"abc123",
			"def456",
			Some("test-name".to_string()),
		)
		.unwrap();

		let json_map = git_source.to_standard_json().unwrap();
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "git");
		assert_eq!(
			json_map.get("remote").unwrap().as_str().unwrap(),
			"https://github.com/user/repo.git"
		);
		assert_eq!(json_map.get("referenced_revision").unwrap().as_str().unwrap(), "abc123");
		assert_eq!(json_map.get("current_revision").unwrap().as_str().unwrap(), "def456");
		assert!(json_map.contains_key("path_pattern"));
		assert_eq!(json_map.get("name").unwrap().as_str().unwrap(), "test-name");
	}

	#[test]
	fn test_to_standard_json_without_name() {
		let git_source = GitSource::try_new(
			"https://github.com/user/repo.git",
			"src/main.rs",
			"abc123",
			"def456",
			None,
		)
		.unwrap();

		let json_map = git_source.to_standard_json().unwrap();
		assert!(json_map.contains_key("name"));
		assert!(!json_map.get("name").unwrap().as_str().unwrap().is_empty());
	}

	#[test]
	fn test_to_above_doc_attr() {
		let git_source = GitSource::try_new(
			"https://github.com/user/repo.git",
			"src/main.rs",
			"abc123",
			"def456",
			Some("test-name".to_string()),
		)
		.unwrap();

		let doc_attr = git_source.to_above_doc_attr().unwrap();
		assert_eq!(doc_attr.source_type, "git");

		// Parse the JSON content to verify it's valid
		let json_value: serde_json::Value = serde_json::from_str(&doc_attr.json_content).unwrap();
		assert_eq!(json_value["src"], "git");
		assert_eq!(json_value["remote"], "https://github.com/user/repo.git");
		assert_eq!(json_value["referenced_revision"], "abc123");
		assert_eq!(json_value["current_revision"], "def456");
		assert!(json_value["path_pattern"].is_object());
		assert_eq!(json_value["name"], "test-name");
	}

	#[test]
	fn test_roundtrip_kwargs_to_json_to_kwargs() {
		let mut original_kwargs = HashMap::new();
		original_kwargs.insert("remote".to_string(), json!("https://github.com/user/repo.git"));
		original_kwargs.insert("ref_rev".to_string(), json!("abc123"));
		original_kwargs.insert("cur_rev".to_string(), json!("def456"));
		original_kwargs.insert("path".to_string(), json!("src/main.rs"));
		original_kwargs.insert("name".to_string(), json!("test-name"));

		// Create GitSource from kwargs
		let git_source = GitSource::from_kwarg_json(&original_kwargs).unwrap();

		// Convert back to JSON
		let json_map = git_source.to_standard_json().unwrap();

		// Verify the JSON contains expected fields
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "git");
		assert_eq!(
			json_map.get("remote").unwrap().as_str().unwrap(),
			"https://github.com/user/repo.git"
		);
		assert_eq!(json_map.get("referenced_revision").unwrap().as_str().unwrap(), "abc123");
		assert_eq!(json_map.get("current_revision").unwrap().as_str().unwrap(), "def456");
		assert!(json_map.contains_key("path_pattern"));
		assert_eq!(json_map.get("name").unwrap().as_str().unwrap(), "test-name");
	}

	#[test]
	fn test_direct_serialization_deserialization() {
		// Create a GitSource using the constructor
		let original = GitSource::try_new(
			"https://github.com/user/repo.git",
			"src/main.rs",
			"abc123",
			"def456",
			Some("test-name".to_string()),
		)
		.unwrap();

		// Serialize to JSON using direct serialization
		let json_map = original.to_standard_json().unwrap();

		// Verify it contains the expected fields from direct serialization
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "git");
		assert!(json_map.contains_key("remote"));
		assert!(json_map.contains_key("referenced_revision"));
		assert!(json_map.contains_key("current_revision"));
		assert!(json_map.contains_key("path_pattern"));
		assert!(json_map.contains_key("name"));
	}

	#[test]
	fn test_direct_deserialization_from_serialized_json() {
		// Create a GitSource using the constructor
		let original = GitSource::try_new(
			"https://github.com/user/repo.git",
			"src/main.rs",
			"abc123",
			"def456",
			Some("test-name".to_string()),
		)
		.unwrap();

		// Serialize to JSON
		let json_map = original.to_standard_json().unwrap();

		// Convert back to HashMap (simulating kwargs)
		let mut kwargs = HashMap::new();
		for (key, value) in json_map {
			if key != "src" {
				// Remove src field as it's not part of the struct
				kwargs.insert(key, value);
			}
		}

		// Try to deserialize using direct deserialization
		let deserialized = GitSource::from_kwarg_json(&kwargs).unwrap();

		// Verify the deserialized version matches the original
		assert_eq!(deserialized.remote, original.remote);
		assert_eq!(deserialized.referenced_revision, original.referenced_revision);
		assert_eq!(deserialized.current_revision, original.current_revision);
		assert_eq!(deserialized.path_pattern.to_string(), original.path_pattern.to_string());
		assert_eq!(deserialized.name, original.name);
	}

	#[test]
	fn test_fallback_to_manual_extraction() {
		// Test that manual extraction still works for legacy syntax
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo.git"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		kwargs.insert("cur_rev".to_string(), json!("def456"));
		kwargs.insert("path".to_string(), json!("src/main.rs"));

		// This should use manual extraction since the kwargs don't match the struct exactly
		let git_source = GitSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(git_source.remote, "https://github.com/user/repo.git");
		assert_eq!(git_source.referenced_revision, "abc123");
		assert_eq!(git_source.current_revision, "def456");
		assert_eq!(git_source.path_pattern.to_string(), "src/main.rs");
	}

	#[test]
	fn test_direct_deserialization_from_standard_format() {
		// Create a GitSource and serialize it to get the exact format
		let original = GitSource::try_new(
			"https://github.com/user/repo.git",
			"src/main.rs",
			"abc123",
			"def456",
			Some("test-name".to_string()),
		)
		.unwrap();

		// Get the serialized format
		let json_map = original.to_standard_json().unwrap();

		// Convert to kwargs (remove src field)
		let mut kwargs = HashMap::new();
		for (key, value) in json_map {
			if key != "src" {
				kwargs.insert(key, value);
			}
		}

		// This should use direct deserialization
		let git_source = GitSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(git_source.remote, "https://github.com/user/repo.git");
		assert_eq!(git_source.referenced_revision, "abc123");
		assert_eq!(git_source.current_revision, "def456");
		assert_eq!(git_source.path_pattern.to_string(), "src/main.rs");
		assert_eq!(git_source.name, "test-name");
	}

	#[test]
	fn test_is_valid_attr_key() {
		// Test valid direct serde fields
		assert!(GitSource::is_valid_attr_key("id"));
		assert!(GitSource::is_valid_attr_key("remote"));
		assert!(GitSource::is_valid_attr_key("path_pattern"));
		assert!(GitSource::is_valid_attr_key("referenced_revision"));
		assert!(GitSource::is_valid_attr_key("current_revision"));
		assert!(GitSource::is_valid_attr_key("name"));
		assert!(GitSource::is_valid_attr_key("formatted_url"));
		assert!(GitSource::is_valid_attr_key("repository_builder"));

		// Test valid legacy ergonomic fields
		assert!(GitSource::is_valid_attr_key("ref_rev"));
		assert!(GitSource::is_valid_attr_key("cur_rev"));
		assert!(GitSource::is_valid_attr_key("path"));

		// Test valid citation-level fields
		assert!(GitSource::is_valid_attr_key("src"));
		assert!(GitSource::is_valid_attr_key("reason"));
		assert!(GitSource::is_valid_attr_key("level"));
		assert!(GitSource::is_valid_attr_key("annotation"));

		// Test invalid fields
		assert!(!GitSource::is_valid_attr_key("invalid_attr"));
		assert!(!GitSource::is_valid_attr_key("unknown_field"));
		assert!(!GitSource::is_valid_attr_key("url"));
		assert!(!GitSource::is_valid_attr_key("match"));
		assert!(!GitSource::is_valid_attr_key("same"));
		assert!(!GitSource::is_valid_attr_key("changed"));
	}
}
