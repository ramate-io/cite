//! HTTP/Hypertext Source Implementation for Citation Macros
//!
//! This module provides parsing and construction of HypertextMatch sources within
//! the `#[cite]` procedural macro. It handles the translation from macro syntax
//! to actual HypertextMatch objects that can validate web content at compile time.
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
//! 3. **Source Construction**: Create HypertextMatch instances using cite-hypertext
//!
//! This separation allows the parsing logic to focus on syntax while delegating
//! the actual HTTP source creation to the hypertext library.

use syn::Expr;
use cite_hypertext::HypertextMatch;

mod macro_syntax;

/// Try to construct a HypertextMatch from the citation expression
/// 
/// This function is kept for backwards compatibility but is deprecated
/// in favor of the keyword argument parsing approach.
pub fn try_construct_http_source_from_expr(_expr: &Expr) -> Option<HypertextMatch> {
    // This function is deprecated in favor of try_construct_http_source_from_citation_args
    // but kept for backwards compatibility with any remaining direct expression parsing
    None
}

/// Try to construct a HypertextMatch from citation arguments using keyword syntax
/// 
/// Supports syntax like:
/// - `http, url = "https://example.com", pattern = "regex"`
/// - `http, url = "https://example.com", selector = "h1"`  
/// - `http, url = "https://example.com", match_type = "full"`
pub fn try_construct_http_source_from_citation_args(args: &[Expr]) -> Option<HypertextMatch> {
    macro_syntax::try_parse_from_citation_args(args)
}
