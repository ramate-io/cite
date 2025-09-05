use crate::{CurrentHttp, HttpDiff, HttpMatch, MatchExpression, ReferencedHttp};
use cite_core::ui::{AboveDocAttr, SourceUi, SourceUiError};
use serde_json::{Map, Value};
use std::collections::HashMap;

impl SourceUi<ReferencedHttp, CurrentHttp, HttpDiff> for HttpMatch {
	fn from_kwarg_json(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError> {
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

	fn to_standard_json(&self) -> Result<Map<String, Value>, SourceUiError> {
		let mut map = Map::new();
		map.insert("src".to_string(), Value::String("http".to_string()));
		map.insert("url".to_string(), Value::String(self.source_url.as_str().to_string()));

		// Serialize match expression
		let match_value = match &self.matches {
			MatchExpression::Regex(pattern) => {
				let mut match_obj = Map::new();
				match_obj.insert("type".to_string(), Value::String("regex".to_string()));
				match_obj.insert("pattern".to_string(), Value::String(pattern.clone()));
				Value::Object(match_obj)
			}
			MatchExpression::CssSelector(selector) => {
				let mut match_obj = Map::new();
				match_obj.insert("type".to_string(), Value::String("css".to_string()));
				match_obj.insert("pattern".to_string(), Value::String(selector.clone()));
				Value::Object(match_obj)
			}
			MatchExpression::XPath(expression) => {
				let mut match_obj = Map::new();
				match_obj.insert("type".to_string(), Value::String("xpath".to_string()));
				match_obj.insert("pattern".to_string(), Value::String(expression.clone()));
				Value::Object(match_obj)
			}
			MatchExpression::Fragment(fragment) => {
				let mut match_obj = Map::new();
				match_obj.insert("type".to_string(), Value::String("fragment".to_string()));
				match_obj.insert("pattern".to_string(), Value::String(fragment.clone()));
				Value::Object(match_obj)
			}
			MatchExpression::FullDocument => {
				let mut match_obj = Map::new();
				match_obj.insert("type".to_string(), Value::String("full".to_string()));
				Value::Object(match_obj)
			}
		};
		map.insert("match".to_string(), match_value);

		map.insert("name".to_string(), Value::String(self.id.as_str().to_string()));

		Ok(map)
	}

	fn to_above_doc_attr(&self) -> Result<AboveDocAttr, SourceUiError> {
		let json_map = self.to_standard_json()?;
		let json_content = serde_json::to_string_pretty(&json_map).map_err(|e| {
			SourceUiError::Serialization(format!("Failed to serialize to JSON: {}", e))
		})?;

		Ok(AboveDocAttr::new(json_content, "http".to_string()))
	}
}
