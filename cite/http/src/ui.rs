use crate::{CurrentHttp, HttpDiff, HttpMatch, MatchExpression, ReferencedHttp};
use cite_core::ui::{AboveDocAttr, SourceUi, SourceUiError};
use serde_json::{Map, Value};
use std::collections::HashMap;

impl SourceUi<ReferencedHttp, CurrentHttp, HttpDiff> for HttpMatch {
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
			SourceUiError::Serialization(format!("Failed to serialize HttpMatch: {}", e))
		})?;

		let mut map = json_value
			.as_object()
			.ok_or_else(|| {
				SourceUiError::Serialization(
					"HttpMatch serialization did not produce an object".to_string(),
				)
			})?
			.clone();

		// Add the src field for consistency
		map.insert("src".to_string(), Value::String("http".to_string()));

		Ok(map)
	}

	fn to_above_doc_attr(&self) -> Result<AboveDocAttr, SourceUiError> {
		let json_map = self.to_standard_json()?;
		let json_content = serde_json::to_string_pretty(&json_map).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to serialize to JSON: {}", e))
		})?;

		Ok(AboveDocAttr::new(json_content, "http".to_string()))
	}

	fn is_valid_attr_key(attr_key: &str) -> bool {
		match attr_key {
			// Direct serde fields
			"matches" | "source_url" | "cache_path" | "id" | "cache" | "cache_behavior" |
			// Legacy ergonomic fields
			"url" | "match" | "pattern" | "selector" | "match_type" | "fragment" |
			// Citation-level fields
			"src" | "reason" | "level" | "annotation" => true,
			_ => false,
		}
	}
}

impl HttpMatch {
	/// Try to deserialize HttpMatch directly from kwargs using serde
	fn try_direct_deserialization(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// Convert HashMap to JSON and try to deserialize
		let json_value = serde_json::to_value(kwargs).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to convert kwargs to JSON: {}", e))
		})?;

		// Try to deserialize as HttpMatch directly
		let source: HttpMatch = serde_json::from_value(json_value).map_err(|e| {
			SourceUiError::Serialization(format!("Direct deserialization failed: {}", e))
		})?;

		Ok(source)
	}

	/// Try manual parameter extraction for backward compatibility
	fn try_manual_extraction(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
		// Extract required parameters
		let url = kwargs
			.get("url")
			.and_then(|v| v.as_str())
			.ok_or_else(|| SourceUiError::MissingParameter("url".to_string()))?;

		// Extract optional match expression
		let match_expr = if let Some(match_value) = kwargs.get("match") {
			match match_value {
				Value::String(s) => {
					if s.starts_with("regex:") {
						MatchExpression::regex(&s[6..])
					} else if s.starts_with("css:") {
						MatchExpression::css_selector(&s[4..])
					} else if s.starts_with("xpath:") {
						MatchExpression::xpath(&s[6..])
					} else if s.starts_with("fragment:") {
						MatchExpression::fragment(&s[9..])
					} else if s == "full" {
						MatchExpression::full_document()
					} else {
						// Default to CSS selector if no prefix
						MatchExpression::css_selector(s)
					}
				}
				Value::Object(obj) => {
					// Support structured match expressions
					if let Some(pattern) = obj.get("pattern").and_then(|v| v.as_str()) {
						if let Some(match_type) = obj.get("type").and_then(|v| v.as_str()) {
							match match_type {
								"regex" => MatchExpression::regex(pattern),
								"css" => MatchExpression::css_selector(pattern),
								"xpath" => MatchExpression::xpath(pattern),
								"fragment" => MatchExpression::fragment(pattern),
								"full" => MatchExpression::full_document(),
								_ => {
									return Err(SourceUiError::InvalidParameter(format!(
										"Unknown match type: {}",
										match_type
									)))
								}
							}
						} else {
							return Err(SourceUiError::MissingParameter("match.type".to_string()));
						}
					} else {
						return Err(SourceUiError::MissingParameter("match.pattern".to_string()));
					}
				}
				_ => {
					return Err(SourceUiError::InvalidParameter(
						"Invalid match expression format".to_string(),
					))
				}
			}
		} else {
			MatchExpression::full_document()
		};

		// Extract optional cache behavior
		let cache_behavior = kwargs
			.get("cache")
			.and_then(|v| v.as_str())
			.map(|s| match s {
				"ignored" => cite_cache::CacheBehavior::Ignored,
				"enabled" => cite_cache::CacheBehavior::Enabled,
				_ => cite_cache::CacheBehavior::Enabled,
			})
			.unwrap_or(cite_cache::CacheBehavior::Enabled);

		// Create the HttpMatch
		HttpMatch::with_match_expression_and_cache_behavior(url, match_expr, cache_behavior)
			.map_err(|e| SourceUiError::Internal(e.into()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_from_kwarg_json_basic_url() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::FullDocument));
	}

	#[test]
	fn test_from_kwarg_json_with_regex_match() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert("match".to_string(), json!("regex:.*"));

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::Regex(_)));
	}

	#[test]
	fn test_from_kwarg_json_with_css_match() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert("match".to_string(), json!("css:.content"));

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::CssSelector(_)));
	}

	#[test]
	fn test_from_kwarg_json_with_xpath_match() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert("match".to_string(), json!("xpath://div[@class='content']"));

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::XPath(_)));
	}

	#[test]
	fn test_from_kwarg_json_with_fragment_match() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert("match".to_string(), json!("fragment:main-content"));

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::Fragment(_)));
	}

	#[test]
	fn test_from_kwarg_json_with_full_match() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert("match".to_string(), json!("full"));

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::FullDocument));
	}

	#[test]
	fn test_from_kwarg_json_with_structured_match() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert(
			"match".to_string(),
			json!({
				"type": "regex",
				"pattern": ".*"
			}),
		);

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::Regex(_)));
	}

	#[test]
	fn test_from_kwarg_json_with_cache_ignored() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert("cache".to_string(), json!("ignored"));

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		// Note: We can't easily test cache behavior without exposing internal state
	}

	#[test]
	fn test_from_kwarg_json_with_cache_enabled() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert("cache".to_string(), json!("enabled"));

		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
	}

	#[test]
	fn test_from_kwarg_json_missing_url() {
		let mut kwargs = HashMap::new();
		kwargs.insert("match".to_string(), json!("regex:.*"));

		let result = HttpMatch::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		if let Err(SourceUiError::MissingParameter(msg)) = result {
			assert_eq!(msg, "url");
		} else {
			panic!("Expected MissingParameter error for url");
		}
	}

	#[test]
	fn test_from_kwarg_json_invalid_match_type() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert(
			"match".to_string(),
			json!({
				"type": "invalid",
				"pattern": ".*"
			}),
		);

		let result = HttpMatch::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		if let Err(SourceUiError::InvalidParameter(msg)) = result {
			assert!(msg.contains("Unknown match type: invalid"));
		} else {
			panic!("Expected InvalidParameter error for invalid match type");
		}
	}

	#[test]
	fn test_from_kwarg_json_missing_match_pattern() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert(
			"match".to_string(),
			json!({
				"type": "regex"
			}),
		);

		let result = HttpMatch::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		if let Err(SourceUiError::MissingParameter(msg)) = result {
			assert_eq!(msg, "match.pattern");
		} else {
			panic!("Expected MissingParameter error for missing pattern");
		}
	}

	#[test]
	fn test_from_kwarg_json_missing_match_type() {
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert(
			"match".to_string(),
			json!({
				"pattern": ".*"
			}),
		);

		let result = HttpMatch::from_kwarg_json(&kwargs);
		assert!(result.is_err());
		if let Err(SourceUiError::MissingParameter(msg)) = result {
			assert_eq!(msg, "match.type");
		} else {
			panic!("Expected MissingParameter error for missing type");
		}
	}

	#[test]
	fn test_to_standard_json_basic() {
		let http_match = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::full_document(),
			cite_cache::CacheBehavior::Enabled,
		)
		.unwrap();

		let json_map = http_match.to_standard_json().unwrap();
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "http");
		assert!(json_map.contains_key("matches"));
		assert!(json_map.contains_key("source_url"));
		assert!(json_map.contains_key("id"));
	}

	#[test]
	fn test_to_standard_json_with_regex() {
		let http_match = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::regex(".*"),
			cite_cache::CacheBehavior::Enabled,
		)
		.unwrap();

		let json_map = http_match.to_standard_json().unwrap();
		let matches_obj = json_map.get("matches").unwrap().as_object().unwrap();
		assert_eq!(matches_obj.get("Regex").unwrap().as_str().unwrap(), ".*");
	}

	#[test]
	fn test_to_standard_json_with_css() {
		let http_match = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::css_selector(".content"),
			cite_cache::CacheBehavior::Enabled,
		)
		.unwrap();

		let json_map = http_match.to_standard_json().unwrap();
		let matches_obj = json_map.get("matches").unwrap().as_object().unwrap();
		assert_eq!(matches_obj.get("CssSelector").unwrap().as_str().unwrap(), ".content");
	}

	#[test]
	fn test_to_standard_json_with_xpath() {
		let http_match = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::xpath("//div[@class='content']"),
			cite_cache::CacheBehavior::Enabled,
		)
		.unwrap();

		let json_map = http_match.to_standard_json().unwrap();
		let matches_obj = json_map.get("matches").unwrap().as_object().unwrap();
		assert_eq!(matches_obj.get("XPath").unwrap().as_str().unwrap(), "//div[@class='content']");
	}

	#[test]
	fn test_to_standard_json_with_fragment() {
		let http_match = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::fragment("main-content"),
			cite_cache::CacheBehavior::Enabled,
		)
		.unwrap();

		let json_map = http_match.to_standard_json().unwrap();
		let matches_obj = json_map.get("matches").unwrap().as_object().unwrap();
		assert_eq!(matches_obj.get("Fragment").unwrap().as_str().unwrap(), "main-content");
	}

	#[test]
	fn test_to_standard_json_with_full_document() {
		let http_match = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::full_document(),
			cite_cache::CacheBehavior::Enabled,
		)
		.unwrap();

		let json_map = http_match.to_standard_json().unwrap();
		assert_eq!(json_map.get("matches").unwrap().as_str().unwrap(), "FullDocument");
	}

	#[test]
	fn test_to_above_doc_attr() {
		let http_match = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::regex(".*"),
			cite_cache::CacheBehavior::Enabled,
		)
		.unwrap();

		let doc_attr = http_match.to_above_doc_attr().unwrap();
		assert_eq!(doc_attr.source_type, "http");

		// Parse the JSON content to verify it's valid and uses direct serialization format
		let json_value: serde_json::Value = serde_json::from_str(&doc_attr.json_content).unwrap();
		assert_eq!(json_value["src"], "http");
		assert!(json_value["source_url"].is_object());
		assert!(json_value["matches"].is_object());
		assert_eq!(json_value["matches"]["Regex"], ".*");
	}

	#[test]
	fn test_roundtrip_kwargs_to_json_to_kwargs() {
		let mut original_kwargs = HashMap::new();
		original_kwargs.insert("url".to_string(), json!("https://example.com"));
		original_kwargs.insert("match".to_string(), json!("regex:.*"));
		original_kwargs.insert("cache".to_string(), json!("enabled"));

		// Create HttpMatch from kwargs (using ergonomic format)
		let http_match = HttpMatch::from_kwarg_json(&original_kwargs).unwrap();

		// Convert back to JSON (should use direct serialization format)
		let json_map = http_match.to_standard_json().unwrap();

		// Verify the JSON contains expected fields in direct serialization format
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "http");
		assert!(json_map.contains_key("source_url"));
		assert!(json_map.contains_key("matches"));
		assert!(json_map.contains_key("cache_behavior"));

		// Verify the matches field contains the regex
		let matches_obj = json_map.get("matches").unwrap().as_object().unwrap();
		assert_eq!(matches_obj.get("Regex").unwrap().as_str().unwrap(), ".*");
	}

	#[test]
	fn test_direct_serialization_deserialization() {
		// Create an HttpMatch using the constructor
		let original = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::regex(".*"),
			cite_cache::CacheBehavior::Enabled,
		)
		.unwrap();

		// Serialize to JSON using direct serialization
		let json_map = original.to_standard_json().unwrap();

		// Verify it contains the expected fields from direct serialization
		assert_eq!(json_map.get("src").unwrap().as_str().unwrap(), "http");
		assert!(json_map.contains_key("matches"));
		assert!(json_map.contains_key("source_url"));
		assert!(json_map.contains_key("cache_path"));
		assert!(json_map.contains_key("id"));
		assert!(json_map.contains_key("cache_behavior"));
		assert!(json_map.contains_key("cache"));

		// Check that source_url contains the expected URL
		let source_url_obj = json_map.get("source_url").unwrap().as_object().unwrap();
		assert_eq!(source_url_obj.get("url").unwrap().as_str().unwrap(), "https://example.com");
	}

	#[test]
	fn test_direct_deserialization_from_serialized_json() {
		// Create an HttpMatch using the constructor
		let original = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::regex(".*"),
			cite_cache::CacheBehavior::Enabled,
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
		let deserialized = HttpMatch::from_kwarg_json(&kwargs).unwrap();

		// Verify the deserialized version matches the original
		assert_eq!(deserialized.source_url.as_str(), original.source_url.as_str());
		assert_eq!(deserialized.matches, original.matches);
		assert_eq!(deserialized.cache_behavior, original.cache_behavior);
		assert_eq!(deserialized.cache_path, original.cache_path);
		assert_eq!(deserialized.id.as_str(), original.id.as_str());
	}

	#[test]
	fn test_fallback_to_manual_extraction() {
		// Test that manual extraction still works for legacy syntax
		let mut kwargs = HashMap::new();
		kwargs.insert("url".to_string(), json!("https://example.com"));
		kwargs.insert("match".to_string(), json!("regex:.*"));

		// This should use manual extraction since the kwargs don't match the struct exactly
		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::Regex(_)));
	}

	#[test]
	fn test_direct_deserialization_from_standard_format() {
		// Create an HttpMatch and serialize it to get the exact format
		let original = HttpMatch::with_match_expression_and_cache_behavior(
			"https://example.com",
			MatchExpression::regex(".*"),
			cite_cache::CacheBehavior::Enabled,
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
		let http_match = HttpMatch::from_kwarg_json(&kwargs).unwrap();
		assert_eq!(http_match.source_url.as_str(), "https://example.com");
		assert!(matches!(http_match.matches, MatchExpression::Regex(_)));
	}

	#[test]
	fn test_is_valid_attr_key() {
		// Test valid direct serde fields
		assert!(HttpMatch::is_valid_attr_key("matches"));
		assert!(HttpMatch::is_valid_attr_key("source_url"));
		assert!(HttpMatch::is_valid_attr_key("cache_path"));
		assert!(HttpMatch::is_valid_attr_key("id"));
		assert!(HttpMatch::is_valid_attr_key("cache"));
		assert!(HttpMatch::is_valid_attr_key("cache_behavior"));

		// Test valid legacy ergonomic fields
		assert!(HttpMatch::is_valid_attr_key("url"));
		assert!(HttpMatch::is_valid_attr_key("match"));
		assert!(HttpMatch::is_valid_attr_key("pattern"));
		assert!(HttpMatch::is_valid_attr_key("selector"));
		assert!(HttpMatch::is_valid_attr_key("match_type"));
		assert!(HttpMatch::is_valid_attr_key("fragment"));

		// Test valid citation-level fields
		assert!(HttpMatch::is_valid_attr_key("src"));
		assert!(HttpMatch::is_valid_attr_key("reason"));
		assert!(HttpMatch::is_valid_attr_key("level"));
		assert!(HttpMatch::is_valid_attr_key("annotation"));

		// Test invalid fields
		assert!(!HttpMatch::is_valid_attr_key("invalid_attr"));
		assert!(!HttpMatch::is_valid_attr_key("unknown_field"));
		assert!(!HttpMatch::is_valid_attr_key("remote"));
		assert!(!HttpMatch::is_valid_attr_key("path"));
		assert!(!HttpMatch::is_valid_attr_key("same"));
		assert!(!HttpMatch::is_valid_attr_key("changed"));
	}
}
