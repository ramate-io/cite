//! Keyword Argument Parsing for HTTP Sources
//!
//! This module implements the parsing logic for the keyword argument syntax
//! used with HTTP sources in `#[cite]` attributes.
//!
//! # Parsing Strategy
//!
//! The parser looks for the following patterns in citation arguments:
//!
//! ```text
//! [http, url = "https://example.com", pattern = "regex", ...other_args]
//! [http, url = "https://example.com", selector = "h1", ...other_args]
//! [http, url = "https://example.com", match_type = "full", ...other_args]
//! ```
//!
//! The parsing algorithm:
//! 1. Verify first argument is the identifier `http`
//! 2. Scan remaining arguments for assignment expressions
//! 3. Extract required `url` parameter
//! 4. Match additional parameters to known HTTP options
//! 5. Construct HttpMatch using cite-http
//!
//! # Supported Parameters
//!
//! **Required:**
//! - `url = "https://example.com"` - Target URL to fetch
//!
//! **Match Type (pick one):**
//! - `pattern = "regex"` - Regex pattern for content extraction
//! - `selector = "h1"` - CSS selector for content extraction
//! - `fragment = "section-id"` - Extract content from specific fragment/section
//! - `match_type = "full"` - Full document (no extraction)
//! - `match_type = "auto"` - Auto-detect fragment from URL (if URL contains #fragment)
//!
//! **Future Extensions:**
//! - `headers = {"Authorization": "Bearer token"}` - Custom HTTP headers
//! - `method = "POST"` - HTTP method (default GET)
//! - `timeout = 30` - Request timeout in seconds
//! - `follow_redirects = true` - Follow HTTP redirects
//!
//! # Error Handling
//!
//! The parser is designed to fail gracefully:
//! - Returns `None` if the syntax doesn't match HTTP source patterns
//! - Allows the main citation parser to try other source types
//! - Validates URLs and match expressions at parse time
//! - Provides helpful error messages for malformed syntax
//!
//! # Examples
//!
//! ```rust,ignore
//! // API endpoint validation
//! #[cite(http, url = "https://api.github.com/repos/user/repo", 
//!        pattern = r#""stargazers_count":\s*(\d+)"#)]
//!
//! // Documentation validation  
//! #[cite(http, url = "https://doc.rust-lang.org/std/", 
//!        selector = "h1")]
//!
//! // Fragment-based validation (manual)
//! #[cite(http, url = "https://example.com/docs", 
//!        fragment = "important-section")]
//!
//! // Fragment-based validation (automatic)
//! #[cite(http, url = "https://example.com/docs#important-section", 
//!        match_type = "auto")]
//!
//! // Even simpler - just URL with fragment (auto-detects)
//! #[cite(http, url = "https://example.com/docs#important-section")]
//!
//! // Full page validation
//! #[cite(http, url = "https://example.com", 
//!        match_type = "full")]
//! ```

use syn::{Expr, Lit};
use cite_http::{HttpMatch, MatchExpression};

/// Parse the keyword argument syntax for HTTP sources
/// 
/// Supports syntax like:
/// - `http, url = "https://example.com", pattern = "regex"`
/// - `http, url = "https://example.com", selector = "h1"`
/// - `http, url = "https://example.com", match_type = "full"`
pub fn try_parse_from_citation_args(args: &[Expr]) -> Option<HttpMatch> {
    // Look for pattern: http, url = "...", (pattern|selector|match_type) = "..."
    if args.is_empty() {
        return None;
    }
    
    // First argument should be the identifier "http"
    if let Expr::Path(path_expr) = &args[0] {
        if path_expr.path.segments.len() == 1 && path_expr.path.segments[0].ident == "http" {
            // Look through remaining arguments for assignments
            let mut url = None;
            let mut match_expression = None;
            let mut cache_behavior = None; // Will be determined later based on env vars and kwargs
            
            for arg in &args[1..] {
                if let Expr::Assign(assign_expr) = arg {
                    if let Expr::Path(left_path) = &*assign_expr.left {
                        if left_path.path.segments.len() == 1 {
                            let name = &left_path.path.segments[0].ident.to_string();
                            
                            match name.as_str() {
                                "url" => {
                                    if let Some(url_str) = extract_string_literal(&assign_expr.right) {
                                        url = Some(url_str);
                                    }
                                }
                                "pattern" => {
                                    if let Some(pattern_str) = extract_string_literal(&assign_expr.right) {
                                        match_expression = Some(MatchExpression::regex(&pattern_str));
                                    }
                                }
                                "selector" => {
                                    if let Some(selector_str) = extract_string_literal(&assign_expr.right) {
                                        match_expression = Some(MatchExpression::css_selector(&selector_str));
                                    }
                                }
                                "match_type" => {
                                    if let Some(match_type_str) = extract_string_literal(&assign_expr.right) {
                                        match match_type_str.as_str() {
                                            "full" => {
                                                match_expression = Some(MatchExpression::full_document());
                                            }
                                            "auto" => {
                                                // Will be handled later with auto-fragment detection
                                                match_expression = Some(MatchExpression::full_document()); // placeholder
                                            }
                                            _ => {
                                                // Unknown match type, skip
                                            }
                                        }
                                    }
                                }
                                "fragment" => {
                                    if let Some(fragment_str) = extract_string_literal(&assign_expr.right) {
                                        match_expression = Some(MatchExpression::fragment(&fragment_str));
                                    }
                                }
                                "cache" => {
                                    if let Some(cache_str) = extract_string_literal(&assign_expr.right) {
                                        match cache_str.as_str() {
                                            "enabled" | "true" => {
                                                cache_behavior = Some(cite_cache::CacheBehavior::Enabled);
                                            }
                                            "disabled" | "false" | "ignored" => {
                                                cache_behavior = Some(cite_cache::CacheBehavior::Ignored);
                                            }
                                            _ => {
                                                // Invalid cache value, ignore
                                            }
                                        }
                                    }
                                }
                                _ => continue, // Unknown parameter, skip
                            }
                        }
                    }
                }
            }
            
            // Construct HttpMatch if we have required parameters
            if let (Some(url_str), match_expr_opt) = (url, match_expression) {
                // Validate URL format at parse time
                if !is_valid_url(&url_str) {
                    return None;
                }
                
                // Use the unified constructor for macro usage
                return HttpMatch::try_new_for_macro(&url_str, match_expr_opt, cache_behavior).ok();
            }
            
            // If we got this far but don't have required params, return None
            // This allows the main parser to show the proper error message
            return None;
        }
    }
    
    None
}

/// Extract a string literal from an expression
fn extract_string_literal(expr: &Expr) -> Option<String> {
    if let Expr::Lit(lit_expr) = expr {
        if let Lit::Str(str_lit) = &lit_expr.lit {
            return Some(str_lit.value());
        }
    }
    None
}

/// Basic URL validation for parse-time checking
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_http_with_pattern() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com"),
            parse_quote!(pattern = r#""test":\s*"([^"]+)""#),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let http_match = result.unwrap();
        assert_eq!(http_match.source_url.as_str(), "https://example.com");
    }

    #[test]
    fn test_parse_http_with_selector() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com"),
            parse_quote!(selector = "h1"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let http_match = result.unwrap();
        assert_eq!(http_match.source_url.as_str(), "https://example.com");
    }

    #[test]
    fn test_parse_http_with_full_document() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com"),
            parse_quote!(match_type = "full"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let http_match = result.unwrap();
        assert_eq!(http_match.source_url.as_str(), "https://example.com");
    }

    #[test]
    fn test_parse_http_missing_url() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(pattern = "test"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_http_missing_match_expression() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_http_invalid_url() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "ftp://example.com"),
            parse_quote!(pattern = "test"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_non_http_source() {
        let args: Vec<Expr> = vec![
            parse_quote!(mock),
            parse_quote!(same = "content"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_url_validation() {
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("http://example.com"));
        assert!(!is_valid_url("ftp://example.com"));
        assert!(!is_valid_url("example.com"));
        assert!(!is_valid_url(""));
    }

    #[test]
    fn test_parse_http_with_fragment() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com"),
            parse_quote!(fragment = "my-section"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let http_match = result.unwrap();
        assert_eq!(http_match.source_url.as_str(), "https://example.com");
    }

    #[test]
    fn test_parse_http_with_auto_fragment() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com/docs#important-section"),
            parse_quote!(match_type = "auto"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let http_match = result.unwrap();
        assert_eq!(http_match.source_url.as_str(), "https://example.com/docs#important-section");
        assert_eq!(http_match.source_url.fragment(), Some("important-section"));
    }

    #[test]
    fn test_parse_http_with_url_fragment_only() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com/docs#section-1"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let http_match = result.unwrap();
        assert_eq!(http_match.source_url.fragment(), Some("section-1"));
        // Should auto-detect fragment matching
        assert!(matches!(http_match.matches, cite_http::MatchExpression::Fragment(_)));
    }

    #[test] 
    fn test_parse_http_no_match_expression_no_fragment() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none(), "Should fail when no match expression and no fragment");
    }
}
