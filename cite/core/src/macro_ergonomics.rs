//! Macro Ergonomics for Cite Sources
//!
//! This module provides convenient functions and utilities for procedural macro
//! implementations that work with the SourceUi trait system.
//!
//! # Design Philosophy
//!
//! The ergonomics module is designed to:
//! - **Simplify macro implementation**: Provide high-level functions for common operations
//! - **Reduce boilerplate**: Handle common patterns like error conversion and JSON parsing
//! - **Maintain type safety**: Use the SourceUi trait system for compile-time validation
//! - **Support extensibility**: Easy to add new source types and macro patterns

use crate::ui::{AboveDocAttr, SourceUiError};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Error type for macro operations
#[derive(Debug, thiserror::Error)]
pub enum MacroError {
	#[error("Source UI error: {0}")]
	SourceUi(#[from] SourceUiError),
	
	#[error("JSON parsing error: {0}")]
	JsonParsing(String),
	
	#[error("Unsupported source type: {0}")]
	UnsupportedSourceType(String),
	
	#[error("Macro generation error: {0}")]
	MacroGeneration(String),
}

/// Result type for macro operations
pub type MacroResult<T> = Result<T, MacroError>;

/// Parse kwargs from a JSON string
/// 
/// This is a convenience function for macros that receive kwargs as strings
/// and need to parse them into the HashMap format expected by SourceUi.
///
/// # Arguments
/// * `json_str` - The JSON string containing the kwargs
///
/// # Returns
/// * `HashMap<String, Value>` - The parsed kwargs
pub fn parse_kwargs_from_json(json_str: &str) -> MacroResult<HashMap<String, Value>> {
	serde_json::from_str(json_str)
		.map_err(|e| MacroError::JsonParsing(format!("Failed to parse JSON: {}", e)))
}

/// Create kwargs from individual parameters (convenience function)
/// 
/// This function helps macros build kwargs maps from individual parameters,
/// which is common when parsing macro arguments.
///
/// # Arguments
/// * `params` - A slice of (key, value) tuples
///
/// # Returns
/// * `HashMap<String, Value>` - The constructed kwargs map
pub fn create_kwargs_from_params(params: &[(&str, Value)]) -> HashMap<String, Value> {
	params.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

/// Validate that required parameters are present in kwargs
/// 
/// This function helps macros validate that all required parameters are present
/// before attempting to create a source.
///
/// # Arguments
/// * `kwargs` - The kwargs to validate
/// * `required_params` - A slice of required parameter names
///
/// # Returns
/// * `MacroResult<()>` - Ok if all required params are present, Err otherwise
pub fn validate_required_params(
	kwargs: &HashMap<String, Value>,
	required_params: &[&str],
) -> MacroResult<()> {
	for param in required_params {
		if !kwargs.contains_key(*param) {
			return Err(MacroError::MacroGeneration(format!(
				"Missing required parameter: {}",
				param
			)));
		}
	}
	Ok(())
}

/// Helper function to create a standard JSON representation from kwargs
/// 
/// This function creates a standardized JSON representation that can be used
/// for generating doc attributes. It's a utility function that doesn't depend
/// on specific source types.
///
/// # Arguments
/// * `source_type` - The type of source ("git", "http", "mock")
/// * `kwargs` - The keyword arguments as a JSON map
///
/// # Returns
/// * `Map<String, Value>` - The standardized JSON representation
pub fn create_standard_json_from_kwargs(
	source_type: &str,
	kwargs: &HashMap<String, Value>,
) -> MacroResult<Map<String, Value>> {
	let mut map = Map::new();
	map.insert("src".to_string(), Value::String(source_type.to_string()));
	
	// Copy all other parameters
	for (key, value) in kwargs {
		if key != "src" {
			map.insert(key.clone(), value.clone());
		}
	}
	
	Ok(map)
}

/// Generate a doc attribute string from JSON data
/// 
/// This function creates the HTML-formatted doc attribute string that can be
/// embedded in Rust doc comments.
///
/// # Arguments
/// * `json_data` - The JSON data to embed
///
/// # Returns
/// * `String` - The formatted doc attribute string
pub fn generate_doc_attr_string(json_data: &Map<String, Value>) -> MacroResult<String> {
	let json_content = serde_json::to_string_pretty(json_data)
		.map_err(|e| MacroError::JsonParsing(format!("Failed to serialize JSON: {}", e)))?;
	
	Ok(format!(
		"<div style=\"display: none;\"><cite above content [{}] end_content/></div>",
		json_content
	))
}

/// Generate a source-specific doc attribute with error handling
/// 
/// This is a high-level function that combines source creation, validation,
/// and doc attribute generation with proper error handling for macro contexts.
/// It works with any source type by using the standardized JSON format.
///
/// # Arguments
/// * `source_type` - The type of source ("git", "http", "mock")
/// * `kwargs` - The keyword arguments as a JSON map
/// * `required_params` - Required parameters for validation
///
/// # Returns
/// * `AboveDocAttr` - The formatted doc attribute content
pub fn generate_source_doc_attr(
	source_type: &str,
	kwargs: &HashMap<String, Value>,
	required_params: &[&str],
) -> MacroResult<AboveDocAttr> {
	// Validate required parameters
	validate_required_params(kwargs, required_params)?;
	
	// Create standardized JSON
	let json_data = create_standard_json_from_kwargs(source_type, kwargs)?;
	
	// Generate doc attribute string
	let json_content = serde_json::to_string_pretty(&json_data)
		.map_err(|e| MacroError::JsonParsing(format!("Failed to serialize JSON: {}", e)))?;
	
	Ok(AboveDocAttr::new(json_content, source_type.to_string()))
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_create_kwargs_from_params() {
		let params = &[
			("remote", json!("https://github.com/user/repo")),
			("ref_rev", json!("abc123")),
			("cur_rev", json!("def456")),
			("path", json!("README.md")),
		];
		
		let kwargs = create_kwargs_from_params(params);
		
		assert_eq!(kwargs.get("remote").unwrap().as_str().unwrap(), "https://github.com/user/repo");
		assert_eq!(kwargs.get("ref_rev").unwrap().as_str().unwrap(), "abc123");
		assert_eq!(kwargs.get("cur_rev").unwrap().as_str().unwrap(), "def456");
		assert_eq!(kwargs.get("path").unwrap().as_str().unwrap(), "README.md");
	}

	#[test]
	fn test_validate_required_params() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		
		// Should pass with all required params
		let result = validate_required_params(&kwargs, &["remote", "ref_rev"]);
		assert!(result.is_ok());
		
		// Should fail with missing params
		let result = validate_required_params(&kwargs, &["remote", "ref_rev", "path"]);
		assert!(result.is_err());
	}

	#[test]
	fn test_parse_kwargs_from_json() {
		let json_str = r#"{"remote": "https://github.com/user/repo", "ref_rev": "abc123"}"#;
		let kwargs = parse_kwargs_from_json(json_str).unwrap();
		
		assert_eq!(kwargs.get("remote").unwrap().as_str().unwrap(), "https://github.com/user/repo");
		assert_eq!(kwargs.get("ref_rev").unwrap().as_str().unwrap(), "abc123");
	}

	#[test]
	fn test_create_standard_json_from_kwargs() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		
		let json_data = create_standard_json_from_kwargs("git", &kwargs).unwrap();
		
		assert_eq!(json_data.get("src").unwrap().as_str().unwrap(), "git");
		assert_eq!(json_data.get("remote").unwrap().as_str().unwrap(), "https://github.com/user/repo");
		assert_eq!(json_data.get("ref_rev").unwrap().as_str().unwrap(), "abc123");
	}

	#[test]
	fn test_generate_source_doc_attr() {
		let mut kwargs = HashMap::new();
		kwargs.insert("remote".to_string(), json!("https://github.com/user/repo"));
		kwargs.insert("ref_rev".to_string(), json!("abc123"));
		kwargs.insert("cur_rev".to_string(), json!("def456"));
		kwargs.insert("path".to_string(), json!("README.md"));
		
		let doc_attr = generate_source_doc_attr("git", &kwargs, &["remote", "ref_rev", "cur_rev", "path"]).unwrap();
		
		assert_eq!(doc_attr.source_type, "git");
		assert!(doc_attr.json_content.contains("git"));
		assert!(doc_attr.json_content.contains("https://github.com/user/repo"));
	}
}
