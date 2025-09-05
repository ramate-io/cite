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

use cite_http::HttpMatch;
use cite_core::ui::SourceUi;
use std::collections::HashMap;
use serde_json::Value;

/// Try to construct an HttpMatch from kwargs using the SourceUi trait
///
/// Supports syntax like:
/// - `url = "https://example.com", match = "regex:pattern"` -> HttpMatch with regex
/// - `url = "https://example.com", match = "css:selector"` -> HttpMatch with CSS selector
/// - `url = "https://example.com", match = "full"` -> HttpMatch with full document
/// - `url = "https://example.com", match = "fragment:section-id"` -> HttpMatch with fragment
/// - `url = "https://example.com#fragment"` -> HttpMatch with auto-detected fragment
pub fn try_get_http_source_from_kwargs(
	kwargs: &HashMap<String, Value>,
) -> Result<HttpMatch, String> {
	HttpMatch::from_kwarg_json(kwargs)
		.map_err(|e| format!("Failed to create HTTP source: {}", e))
}
