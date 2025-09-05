use crate::mock::{CurrentString, MockSource, ReferencedString, StringDiff};
use crate::ui::{AboveDocAttr, SourceUi, SourceUiError};
use serde_json::{Map, Value};
use std::collections::HashMap;

impl SourceUi<ReferencedString, CurrentString, StringDiff> for MockSource {
	fn from_kwarg_json(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// First, try direct deserialization from the kwargs
		if let Ok(source) = Self::try_direct_deserialization(kwargs) {
			return Ok(source);
		}

		// If direct deserialization fails, try legacy syntax patterns
		Self::try_legacy_syntax(kwargs)
	}

	fn to_standard_json(&self) -> Result<Map<String, Value>, SourceUiError> {
		let mut map = Map::new();
		map.insert("src".to_string(), Value::String("mock".to_string()));
		map.insert(
			"referenced_content".to_string(),
			Value::String(self.referenced_content.clone()),
		);
		map.insert("current_content".to_string(), Value::String(self.current_content.clone()));
		map.insert("name".to_string(), Value::String(self.id.as_str().to_string()));

		Ok(map)
	}

	fn to_above_doc_attr(&self) -> Result<AboveDocAttr, SourceUiError> {
		let json_map = self.to_standard_json()?;
		let json_content = serde_json::to_string_pretty(&json_map).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to serialize to JSON: {}", e))
		})?;

		Ok(AboveDocAttr::new(json_content, "mock".to_string()))
	}
}

impl MockSource {
	/// Try to deserialize MockSource directly from kwargs using serde
	fn try_direct_deserialization(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// Convert HashMap to JSON and try to deserialize
		let json_value = serde_json::to_value(kwargs).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to convert kwargs to JSON: {}", e))
		})?;

		// Try to deserialize as MockSource directly
		let source: MockSource = serde_json::from_value(json_value).map_err(|e| {
			SourceUiError::Serialization(format!("Direct deserialization failed: {}", e))
		})?;

		Ok(source)
	}

	/// Try legacy syntax patterns for backward compatibility
	fn try_legacy_syntax(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// Extract required parameters - support both new and legacy syntax
		let referenced = kwargs
			.get("referenced")
			.or_else(|| kwargs.get("referenced_content"))
			.or_else(|| kwargs.get("same")) // Legacy support
			.and_then(|v| v.as_str())
			.ok_or_else(|| {
				SourceUiError::MissingParameter(
					"referenced, referenced_content, or same".to_string(),
				)
			})?
			.to_string();

		let current = kwargs
			.get("current")
			.or_else(|| kwargs.get("current_content"))
			.and_then(|v| v.as_str())
			.map(|s| s.to_string())
			.unwrap_or_else(|| referenced.clone());

		// Handle legacy "changed" syntax
		let (referenced_content, current_content) = if let Some(changed_val) = kwargs.get("changed")
		{
			// Parse the changed tuple from JSON array (legacy syntax)
			if let Some(changed_array) = changed_val.as_array() {
				if changed_array.len() == 2 {
					let old = changed_array[0].as_str().unwrap_or("").to_string();
					let new = changed_array[1].as_str().unwrap_or("").to_string();
					(old, new)
				} else {
					return Err(SourceUiError::InvalidParameter(
						"changed parameter must be a tuple of two strings".to_string(),
					));
				}
			} else {
				return Err(SourceUiError::InvalidParameter(
					"changed parameter must be a tuple of two strings".to_string(),
				));
			}
		} else {
			(referenced, current)
		};

		// Extract optional name parameter
		let name = kwargs.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());

		// Create the MockSource
		let mut source = MockSource::new(referenced_content, current_content);

		// Override the ID if a name was provided
		if let Some(name) = name {
			source.id = crate::Id::new(name);
		}

		Ok(source)
	}
}
