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
		let mut map = Map::new();
		map.insert("src".to_string(), Value::String("git".to_string()));
		map.insert("remote".to_string(), Value::String(self.remote.clone()));
		map.insert("ref_rev".to_string(), Value::String(self.referenced_revision.clone()));
		map.insert("cur_rev".to_string(), Value::String(self.current_revision.clone()));
		map.insert("path".to_string(), Value::String(self.path_pattern.to_string()));
		map.insert("name".to_string(), Value::String(self.name.clone()));

		Ok(map)
	}

	fn to_above_doc_attr(&self) -> Result<AboveDocAttr, SourceUiError> {
		let json_map = self.to_standard_json()?;
		let json_content = serde_json::to_string_pretty(&json_map).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to serialize to JSON: {}", e))
		})?;

		Ok(AboveDocAttr::new(json_content, "git".to_string()))
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
