use crate::mock::{CurrentString, MockSource, ReferencedString, StringDiff};
use crate::ui::{AboveDocAttr, SourceUi, SourceUiError};
use serde_json::{Map, Value};
use std::collections::HashMap;

impl SourceUi<ReferencedString, CurrentString, StringDiff> for MockSource {
	fn from_kwarg_json(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// Check if this is legacy syntax first
		if kwargs.contains_key("same") || kwargs.contains_key("changed") {
			return Self::try_legacy_syntax(kwargs);
		}

		// Try direct deserialization from the kwargs
		if let Ok(source) = Self::try_direct_deserialization(kwargs) {
			return Ok(source);
		}

		// If direct deserialization fails, try legacy syntax patterns as fallback
		Self::try_legacy_syntax(kwargs)
	}

	fn to_standard_json(&self) -> Result<Map<String, Value>, SourceUiError> {
		// Use direct serialization to JSON, then convert to Map
		let json_value = serde_json::to_value(self).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to serialize MockSource: {}", e))
		})?;

		let mut map = json_value
			.as_object()
			.ok_or_else(|| {
				SourceUiError::Serialization(
					"MockSource serialization did not produce an object".to_string(),
				)
			})?
			.clone();

		// Add the src field for consistency
		map.insert("src".to_string(), Value::String("mock".to_string()));

		Ok(map)
	}

	fn to_above_doc_attr(&self) -> Result<AboveDocAttr, SourceUiError> {
		let json_map = self.to_standard_json()?;
		let json_content = serde_json::to_string_pretty(&json_map).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to serialize to JSON: {}", e))
		})?;

		Ok(AboveDocAttr::new(json_content, "mock".to_string()))
	}

	fn is_valid_attr_key(attr_key: &str) -> bool {
		match attr_key {
			// Direct serde fields
			"id" | "referenced_content" | "current_content" |
			// Legacy ergonomic fields
			"same" | "changed" | "referenced" | "current" |
			// Citation-level fields
			"src" | "reason" | "level" | "annotation" => true,
			_ => false,
		}
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
		// Handle legacy "changed" syntax first
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
			// Extract required parameters - support both new and legacy syntax
			let referenced = kwargs
				.get("referenced")
				.or_else(|| kwargs.get("referenced_content"))
				.or_else(|| kwargs.get("same")) // Legacy support
				.and_then(|v| v.as_str())
				.ok_or_else(|| {
					SourceUiError::MissingParameter(
						"referenced, referenced_content, same, or changed".to_string(),
					)
				})?
				.to_string();

			let current = kwargs
				.get("current")
				.or_else(|| kwargs.get("current_content"))
				.and_then(|v| v.as_str())
				.map(|s| s.to_string())
				.unwrap_or_else(|| referenced.clone());

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

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_from_kwarg_json_basic() {
		let mut kwargs = HashMap::new();
		kwargs.insert("referenced_content".to_string(), json!("old content"));
		kwargs.insert("current_content".to_string(), json!("new content"));
		kwargs.insert("id".to_string(), json!("test-id"));

		let mock_source = MockSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(mock_source.referenced_content, "old content");
		assert_eq!(mock_source.current_content, "new content");
		assert_eq!(mock_source.id.as_str(), "test-id");
	}

	#[test]
	fn test_from_kwarg_json_legacy_same() {
		let mut kwargs = HashMap::new();
		kwargs.insert("same".to_string(), json!("same content"));

		let mock_source = MockSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(mock_source.referenced_content, "same content");
		assert_eq!(mock_source.current_content, "same content");
	}

	#[test]
	fn test_from_kwarg_json_legacy_changed() {
		let mut kwargs = HashMap::new();
		kwargs.insert("changed".to_string(), json!(["old", "new"]));

		let mock_source = MockSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(mock_source.referenced_content, "old");
		assert_eq!(mock_source.current_content, "new");
	}

	#[test]
	fn test_from_kwarg_json_legacy_referenced() {
		let mut kwargs = HashMap::new();
		kwargs.insert("referenced".to_string(), json!("referenced content"));

		let mock_source = MockSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(mock_source.referenced_content, "referenced content");
		assert_eq!(mock_source.current_content, "referenced content");
	}

	#[test]
	fn test_from_kwarg_json_with_name() {
		let mut kwargs = HashMap::new();
		kwargs.insert("referenced_content".to_string(), json!("old content"));
		kwargs.insert("current_content".to_string(), json!("new content"));
		kwargs.insert("name".to_string(), json!("custom-name"));

		let mock_source = MockSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(mock_source.id.as_str(), "custom-name");
	}

	#[test]
	fn test_from_kwarg_json_missing_content() {
		let mut kwargs = HashMap::new();
		kwargs.insert("current_content".to_string(), json!("new content"));

		let result = MockSource::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), SourceUiError::MissingParameter(_)));
	}

	#[test]
	fn test_from_kwarg_json_invalid_changed() {
		let mut kwargs = HashMap::new();
		kwargs.insert("changed".to_string(), json!(["single"]));

		let result = MockSource::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), SourceUiError::InvalidParameter(_)));
	}

	#[test]
	fn test_to_standard_json_basic() {
		let mock_source = MockSource::new("old content".to_string(), "new content".to_string());

		let json_map = mock_source.to_standard_json().unwrap();
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "mock");
		assert_eq!(json_map.get("referenced_content").unwrap().as_str().unwrap(), "old content");
		assert_eq!(json_map.get("current_content").unwrap().as_str().unwrap(), "new content");
		assert!(json_map.contains_key("id"));
	}

	#[test]
	fn test_to_standard_json_with_custom_id() {
		let mut mock_source = MockSource::new("old content".to_string(), "new content".to_string());
		mock_source.id = crate::Id::new("custom-id".to_string());

		let json_map = mock_source.to_standard_json().unwrap();
		assert_eq!(json_map.get("id").unwrap().as_str().unwrap(), "custom-id");
	}

	#[test]
	fn test_to_above_doc_attr() {
		let mock_source = MockSource::new("old content".to_string(), "new content".to_string());

		let doc_attr = mock_source.to_above_doc_attr().unwrap();
		assert_eq!(doc_attr.source_type, "mock");

		// Parse the JSON content to verify it's valid
		let json_value: serde_json::Value = serde_json::from_str(&doc_attr.json_content).unwrap();
		assert_eq!(json_value["src"], "mock");
		assert_eq!(json_value["referenced_content"], "old content");
		assert_eq!(json_value["current_content"], "new content");
	}

	#[test]
	fn test_roundtrip_kwargs_to_json_to_kwargs() {
		let mut original_kwargs = HashMap::new();
		original_kwargs.insert("referenced_content".to_string(), json!("old content"));
		original_kwargs.insert("current_content".to_string(), json!("new content"));
		original_kwargs.insert("id".to_string(), json!("test-id"));

		// Create MockSource from kwargs
		let mock_source = MockSource::from_kwarg_json(&original_kwargs).unwrap();

		// Convert back to JSON
		let json_map = mock_source.to_standard_json().unwrap();

		// Verify the JSON contains expected fields
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "mock");
		assert_eq!(json_map.get("referenced_content").unwrap().as_str().unwrap(), "old content");
		assert_eq!(json_map.get("current_content").unwrap().as_str().unwrap(), "new content");
		assert_eq!(json_map.get("id").unwrap().as_str().unwrap(), "test-id");
	}

	#[test]
	fn test_direct_serialization_deserialization() {
		// Create a MockSource using the constructor
		let original = MockSource::new("old content".to_string(), "new content".to_string());

		// Serialize to JSON using direct serialization
		let json_map = original.to_standard_json().unwrap();

		// Verify it contains the expected fields from direct serialization
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "mock");
		assert!(json_map.contains_key("referenced_content"));
		assert!(json_map.contains_key("current_content"));
		assert!(json_map.contains_key("id"));
	}

	#[test]
	fn test_direct_deserialization_from_serialized_json() {
		// Create a MockSource using the constructor
		let original = MockSource::new("old content".to_string(), "new content".to_string());

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
		let deserialized = MockSource::from_kwarg_json(&kwargs).unwrap();

		// Verify the deserialized version matches the original
		assert_eq!(deserialized.referenced_content, original.referenced_content);
		assert_eq!(deserialized.current_content, original.current_content);
		assert_eq!(deserialized.id.as_str(), original.id.as_str());
	}

	#[test]
	fn test_fallback_to_legacy_syntax() {
		// Test that legacy syntax still works when direct deserialization fails
		let mut kwargs = HashMap::new();
		kwargs.insert("same".to_string(), json!("same content"));

		let mock_source = MockSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(mock_source.referenced_content, "same content");
		assert_eq!(mock_source.current_content, "same content");
	}

	#[test]
	fn test_direct_deserialization_from_standard_format() {
		// Create a MockSource and serialize it to get the exact format
		let original = MockSource::new("old content".to_string(), "new content".to_string());

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
		let mock_source = MockSource::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(mock_source.referenced_content, "old content");
		assert_eq!(mock_source.current_content, "new content");
	}

	#[test]
	fn test_is_valid_attr_key() {
		// Test valid direct serde fields
		assert!(MockSource::is_valid_attr_key("id"));
		assert!(MockSource::is_valid_attr_key("referenced_content"));
		assert!(MockSource::is_valid_attr_key("current_content"));

		// Test valid legacy ergonomic fields
		assert!(MockSource::is_valid_attr_key("same"));
		assert!(MockSource::is_valid_attr_key("changed"));
		assert!(MockSource::is_valid_attr_key("referenced"));
		assert!(MockSource::is_valid_attr_key("current"));

		// Test valid citation-level fields
		assert!(MockSource::is_valid_attr_key("src"));
		assert!(MockSource::is_valid_attr_key("reason"));
		assert!(MockSource::is_valid_attr_key("level"));
		assert!(MockSource::is_valid_attr_key("annotation"));

		// Test invalid fields
		assert!(!MockSource::is_valid_attr_key("invalid_attr"));
		assert!(!MockSource::is_valid_attr_key("unknown_field"));
		assert!(!MockSource::is_valid_attr_key("remote"));
		assert!(!MockSource::is_valid_attr_key("url"));
		assert!(!MockSource::is_valid_attr_key("path"));
	}
}
