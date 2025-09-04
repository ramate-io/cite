//! HTTP/Http Source Implementation for Citation Macros
//!
//! This module provides parsing and construction of HttpMatch sources within
//! the `#[cite]` procedural macro. It handles the translation from macro syntax
//! to actual HttpMatch objects that can validate web content at compile time.
//!
//! # Design Rationale
//!
//! HTTP sources enable citations to validate web content, APIs, and documentation:
//!
//! 1. **API Validation**: Ensure external APIs haven't changed structure
//! 2. **Documentation Links**: Verify referenced documentation is still accurate
//! 3. **External Resources**: Track changes in external content dependencies
//!
//! # Syntax Design
//!
//! The HTTP syntax follows the keyword argument pattern:
//!
//! ```rust,ignore
//! #[cite(http, url = "https://api.example.com/v1", pattern = r#""version":\s*"([^"]+)""#)]
//! #[cite(http, url = "https://example.com/docs", selector = "h1")]
//! #[cite(http, url = "https://example.com", match_type = "full")]
//! ```
//!
//! This syntax was chosen because:
//! - **Clear Intent**: URL and match pattern are explicitly separated
//! - **Type Safety**: Different match types (regex, CSS selector, full document)
//! - **Extensibility**: Easy to add new match types and HTTP options
//!
//! # Implementation Strategy
//!
//! The module uses a multi-phase approach:
//!
//! 1. **Syntax Parsing**: Extract HTTP-specific arguments from the citation
//! 2. **Validation**: Validate URLs and match expressions at compile time
//! 3. **Source Construction**: Create HttpMatch instances using cite-http
//!
//! This separation allows the parsing logic to focus on syntax while delegating
//! the actual HTTP source creation to the http library.

use cite_http::{HttpMatch, MatchExpression};

/// Try to construct an HttpMatch from kwargs
///
/// Supports syntax like:
/// - `url = "https://example.com", pattern = "regex"` -> HttpMatch with regex
/// - `url = "https://example.com", selector = "h1"` -> HttpMatch with CSS selector
/// - `url = "https://example.com", match_type = "full"` -> HttpMatch with full document
/// - `url = "https://example.com", fragment = "section-id"` -> HttpMatch with fragment
/// - `url = "https://example.com#fragment"` -> HttpMatch with auto-detected fragment
pub fn try_get_http_source_from_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<HttpMatch, String> {
	// Extract required URL parameter
	let url = kwargs
		.get("url")
		.and_then(|v| v.as_str())
		.ok_or("HTTP source requires 'url' parameter")?;

	// Validate URL format
	if !is_valid_url(url) {
		return Err(format!("Invalid URL format: {}", url));
	}

	// Parse optional parameters
	let pattern = kwargs.get("pattern").and_then(|v| v.as_str());
	let selector = kwargs.get("selector").and_then(|v| v.as_str());
	let match_type = kwargs.get("match_type").and_then(|v| v.as_str());
	let fragment = kwargs.get("fragment").and_then(|v| v.as_str());

	// Determine the match expression based on parameters
	let match_expression = if let Some(mt) = match_type {
		match mt {
			"full" => Some(MatchExpression::full_document()),
			"auto" => None, // Let auto-detection work
			_ => return Err(format!("Unknown match_type: {}", mt)),
		}
	} else if let Some(pat) = pattern {
		Some(MatchExpression::regex(pat))
	} else if let Some(sel) = selector {
		Some(MatchExpression::css_selector(sel))
	} else if let Some(frag) = fragment {
		Some(MatchExpression::fragment(frag))
	} else {
		None // Let auto-detection work
	};

	// Parse cache behavior if provided
	let cache_behavior = kwargs.get("cache").and_then(|v| v.as_str()).map(|cache_str| {
		match cache_str {
			"enabled" | "true" => cite_cache::CacheBehavior::Enabled,
			"disabled" | "false" | "ignored" => cite_cache::CacheBehavior::Ignored,
			_ => cite_cache::CacheBehavior::Enabled, // Default to enabled
		}
	});

	// Construct the HttpMatch with the appropriate parameters
	HttpMatch::try_new_for_macro(url, match_expression, cache_behavior)
		.map_err(|e| format!("Failed to create HTTP source: {:?}", e))
}

/// Basic URL validation for parse-time checking
fn is_valid_url(url: &str) -> bool {
	url.starts_with("http://") || url.starts_with("https://")
}
