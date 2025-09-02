//! Keyword Argument Parsing for Git Sources
//!
//! This module implements the parsing logic for the keyword argument syntax
//! used with Git sources in `#[cite]` attributes.
//!
//! # Parsing Strategy
//!
//! The parser looks for the following patterns in citation arguments:
//!
//! ```text
//! [git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "README.md", ...other_args]
//! [git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/lib.rs#L1-L10", ...other_args]
//! [git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/**/*.rs", ...other_args]
//! ```
//!
//! The parsing algorithm:
//! 1. Verify first argument is the identifier `git`
//! 2. Scan remaining arguments for assignment expressions
//! 3. Extract required `revision` and `path` parameters
//! 4. Match additional parameters to known Git options
//! 5. Construct GitSource using cite-git
//!
//! # Supported Parameters
//!
//! **Required:**
//! - `revision = "74aa653664cd90adcc5f836f1777f265c109045b"` - Git commit hash or reference
//! - `path = "README.md"` - File path relative to repository root
//!
//! **Path Options:**
//! - `path = "src/lib.rs#L1-L10"` - File with line range specification
//! - `path = "src/**/*.rs"` - Glob pattern for multiple files
//! - `path = "README.md#L5"` - Single line specification
//!
//! **Future Extensions:**
//! - `branch = "main"` - Branch name instead of commit hash
//! - `tag = "v1.0.0"` - Tag name instead of commit hash
//! - `remote = "origin"` - Remote name for cross-repository references
//! - `submodule = "path/to/submodule"` - Submodule path
//!
//! # Error Handling
//!
//! The parser is designed to fail gracefully:
//! - Returns `None` if the syntax doesn't match Git source patterns
//! - Allows the main citation parser to try other source types
//! - Validates revision format and path syntax at parse time
//! - Provides helpful error messages for malformed syntax
//!
//! # Examples
//!
//! ```rust,ignore
//! // Basic file validation
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", 
//!        path = "README.md")]
//!
//! // Line range validation
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", 
//!        path = "src/lib.rs#L1-L10")]
//!
//! // Glob pattern validation
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", 
//!        path = "src/**/*.rs")]
//!
//! // Single line validation
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", 
//!        path = "README.md#L5")]
//!
//! // Cross-repository validation (future)
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", 
//!        path = "README.md", remote = "origin")]
//! ```

use syn::{Expr, Lit};
use cite_git::GitSource;

/// Parse the keyword argument syntax for Git sources
/// 
/// Supports syntax like:
/// - `git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "README.md"`
/// - `git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/lib.rs#L1-L10"`
/// - `git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/**/*.rs"`
pub fn try_parse_from_citation_args(args: &[Expr]) -> Option<GitSource> {
    // Look for pattern: git, revision = "...", path = "..."
    if args.is_empty() {
        return None;
    }
    
    // First argument should be the identifier "git"
    if let Expr::Path(path_expr) = &args[0] {
        if path_expr.path.segments.len() == 1 && path_expr.path.segments[0].ident == "git" {
            // Look through remaining arguments for assignments
            let mut revision = None;
            let mut path = None;
            
            for arg in &args[1..] {
                if let Expr::Assign(assign_expr) = arg {
                    if let Expr::Path(left_path) = &*assign_expr.left {
                        if left_path.path.segments.len() == 1 {
                            let name = &left_path.path.segments[0].ident.to_string();
                            
                            match name.as_str() {
                                "revision" => {
                                    if let Some(revision_str) = extract_string_literal(&assign_expr.right) {
                                        revision = Some(revision_str);
                                    }
                                }
                                "path" => {
                                    if let Some(path_str) = extract_string_literal(&assign_expr.right) {
                                        path = Some(path_str);
                                    }
                                }
                                _ => continue, // Unknown parameter, skip
                            }
                        }
                    }
                }
            }
            
            // Construct GitSource if we have required parameters
            if let (Some(revision_str), Some(path_str)) = (revision, path) {
                // Validate revision format at parse time
                if !is_valid_revision(&revision_str) {
                    return None;
                }
                
                // Validate path format at parse time
                if !is_valid_path(&path_str) {
                    return None;
                }
                
                // Use the constructor for macro usage
                return GitSource::try_new(&revision_str, &path_str).ok();
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

/// Basic revision validation for parse-time checking
fn is_valid_revision(revision: &str) -> bool {
    // Git commit hashes are typically 40 characters (SHA-1) or 64 characters (SHA-256)
    // We'll be more lenient and accept any reasonable length
    if revision.len() < 7 {
        return false; // Too short for a meaningful commit hash
    }
    
    // Check if it looks like a hex string (basic validation)
    revision.chars().all(|c| c.is_ascii_hexdigit())
}

/// Basic path validation for parse-time checking
fn is_valid_path(path: &str) -> bool {
    // Debug output for failing case
    if path == "src/lib.rs#L1-L10" {
        println!("Debugging path validation for: '{}'", path);
        println!("  hash_pos: {:?}", path.find('#'));
        if let Some(hash_pos) = path.find('#') {
            let path_part = &path[..hash_pos];
            let line_part = &path[hash_pos + 1..];
            println!("  path_part: '{}'", path_part);
            println!("  line_part: '{}'", line_part);
            println!("  line_part.starts_with('L'): {}", line_part.starts_with('L'));
            if line_part.starts_with('L') {
                let line_range = &line_part[1..];
                println!("  line_range: '{}'", line_range);
                println!("  line_range.contains('-'): {}", line_range.contains('-'));
                if line_range.contains('-') {
                    let parts: Vec<&str> = line_range.split('-').collect();
                    println!("  parts: {:?}", parts);
                    println!("  parts.len(): {}", parts.len());
                    if parts.len() == 2 {
                        println!("  parts[0].chars().all(|c| c.is_ascii_digit()): {}", parts[0].chars().all(|c| c.is_ascii_digit()));
                        println!("  parts[1].chars().all(|c| c.is_ascii_digit()): {}", parts[1].chars().all(|c| c.is_ascii_digit()));
                    }
                }
            }
        }
    }
    // Path should not be empty
    if path.is_empty() {
        return false;
    }
    
    // Path should not start with a slash (relative to repo root)
    if path.starts_with('/') {
        return false;
    }
    
    // Path should not contain invalid characters
    if path.contains('\0') {
        return false;
    }
    
    // If path contains line range specification, validate it
    if let Some(hash_pos) = path.find('#') {
        let path_part = &path[..hash_pos];
        let line_part = &path[hash_pos + 1..];
        
        // Path part should not be empty
        if path_part.is_empty() {
            return false;
        }
        
        // Line part should start with 'L' and contain valid line range
        if !line_part.starts_with('L') {
            return false;
        }
        
        // Basic line range validation
        let line_range = &line_part[1..];
        if line_range.is_empty() {
            return false;
        }
        
        // Check if it's a range (L1-L10) or single line (L5)
        if line_range.contains('-') {
            let parts: Vec<&str> = line_range.split('-').collect();
            if parts.len() != 2 {
                return false;
            }
            // Both parts should be numbers (remove any 'L' prefix from the second part)
            let first_part = parts[0];
            let second_part = if parts[1].starts_with('L') {
                &parts[1][1..]
            } else {
                parts[1]
            };
            
            if !first_part.chars().all(|c| c.is_ascii_digit()) || 
               !second_part.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
        } else {
            // Single line should be a number
            if !line_range.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_git_basic() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
            parse_quote!(path = "README.md"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let git_source = result.unwrap();
        assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
        assert_eq!(git_source.path_pattern.path, "README.md");
    }

    #[test]
    fn test_parse_git_with_line_range() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
            parse_quote!(path = "src/lib.rs#L1-L10"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let git_source = result.unwrap();
        assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
        assert_eq!(git_source.path_pattern.path, "src/lib.rs");
    }

    #[test]
    fn test_parse_git_with_glob_pattern() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
            parse_quote!(path = "src/**/*.rs"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let git_source = result.unwrap();
        assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
        assert_eq!(git_source.path_pattern.path, "src/**/*.rs");
    }

    #[test]
    fn test_parse_git_missing_revision() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(path = "README.md"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_git_missing_path() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_git_invalid_revision() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(revision = "invalid"),
            parse_quote!(path = "README.md"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_git_invalid_path() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
            parse_quote!(path = ""),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_non_git_source() {
        let args: Vec<Expr> = vec![
            parse_quote!(http),
            parse_quote!(url = "https://example.com"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_none());
    }

    #[test]
    fn test_revision_validation() {
        assert!(is_valid_revision("74aa653664cd90adcc5f836f1777f265c109045b"));
        assert!(is_valid_revision("1234567890abcdef1234567890abcdef12345678"));
        assert!(!is_valid_revision("invalid"));
        assert!(!is_valid_revision("123"));
        assert!(!is_valid_revision(""));
    }

    #[test]
    fn test_path_validation() {
        // Valid paths
        assert!(is_valid_path("README.md"));
        assert!(is_valid_path("src/lib.rs"));
        assert!(is_valid_path("src/**/*.rs"));
        assert!(is_valid_path("README.md#L5"));
        
        // Debug the failing case
        let test_path = "src/lib.rs#L1-L10";
        println!("Testing path: '{}'", test_path);
        println!("is_valid_path result: {}", is_valid_path(test_path));
        assert!(is_valid_path("src/lib.rs#L1-L10"));
        
        // Invalid paths
        assert!(!is_valid_path(""));
        assert!(!is_valid_path("/absolute/path"));
        assert!(!is_valid_path("file.rs#L"));
        assert!(!is_valid_path("file.rs#L1-"));
        assert!(!is_valid_path("file.rs#L-10"));
        assert!(!is_valid_path("file.rs#L1-L10-L20"));
        assert!(!is_valid_path("file.rs#La-L10"));
        assert!(!is_valid_path("file.rs#L1-Lb"));
    }

    #[test]
    fn test_parse_git_with_single_line() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
            parse_quote!(path = "README.md#L5"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let git_source = result.unwrap();
        assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
        assert_eq!(git_source.path_pattern.path, "README.md");
    }

    #[test]
    fn test_parse_git_with_complex_path() {
        let args: Vec<Expr> = vec![
            parse_quote!(git),
            parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
            parse_quote!(path = "src/core/behavior.rs#L42-L100"),
        ];
        
        let result = try_parse_from_citation_args(&args);
        assert!(result.is_some());
        
        let git_source = result.unwrap();
        assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
        assert_eq!(git_source.path_pattern.path, "src/core/behavior.rs");
    }
}
