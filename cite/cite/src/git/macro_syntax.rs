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

use cite_git::GitSource;
use syn::{Expr, Lit};

/// Parse the keyword argument syntax for Git sources
///
/// Supports syntax like:
/// - `git, remote = "https://github.com/ramate-io/cite", revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "README.md"`
/// - `git, remote = "https://github.com/ramate-io/cite", revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/lib.rs#L1-L10"`
/// - `git, remote = "https://github.com/ramate-io/cite", revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/**/*.rs"`
pub fn try_parse_from_citation_args(args: &[Expr]) -> Option<GitSource> {
	// Look for pattern: git, remote = "...", revision = "...", path = "..."
	if args.is_empty() {
		return None;
	}

	// First argument should be the identifier "git"
	if let Expr::Path(path_expr) = &args[0] {
		if path_expr.path.segments.len() == 1 && path_expr.path.segments[0].ident == "git" {
			// Look through remaining arguments for assignments
			let mut remote = None;
			let mut revision = None;
			let mut path = None;

			for arg in &args[1..] {
				if let Expr::Assign(assign_expr) = arg {
					if let Expr::Path(left_path) = &*assign_expr.left {
						if left_path.path.segments.len() == 1 {
							let name = &left_path.path.segments[0].ident.to_string();

							match name.as_str() {
								"remote" => {
									if let Some(remote_str) =
										extract_string_literal(&assign_expr.right)
									{
										remote = Some(remote_str);
									}
								}
								"revision" => {
									if let Some(revision_str) =
										extract_string_literal(&assign_expr.right)
									{
										revision = Some(revision_str);
									}
								}
								"path" => {
									if let Some(path_str) =
										extract_string_literal(&assign_expr.right)
									{
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
			if let (Some(remote_str), Some(revision_str), Some(path_str)) = (remote, revision, path)
			{
				// Use the constructor for macro usage - let it handle validation
				return GitSource::try_new(&remote_str, &path_str, &revision_str).ok();
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

#[cfg(test)]
mod tests {
	use super::*;
	use syn::parse_quote;

	#[test]
	fn test_parse_git_basic() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
			parse_quote!(path = "README.md"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "README.md");
		assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
	}

	#[test]
	fn test_parse_git_with_line_range() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
			parse_quote!(path = "src/lib.rs#L1-L10"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "src/lib.rs");
		assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
	}

	#[test]
	fn test_parse_git_with_glob_pattern() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
			parse_quote!(path = "src/**/*.rs"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "src/**/*.rs");
		assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
	}

	#[test]
	fn test_parse_git_missing_revision() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(path = "README.md"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_none());
	}

	#[test]
	fn test_parse_git_missing_path() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_none());
	}

	#[test]
	fn test_parse_git_invalid_revision() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(revision = "invalid"),
			parse_quote!(path = "README.md"),
		];

		let result = try_parse_from_citation_args(&args);
		// The macro syntax should pass through the arguments, validation happens in constructor
		assert!(result.is_some());
	}

	#[test]
	fn test_parse_git_invalid_path() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
			parse_quote!(path = ""),
		];

		let result = try_parse_from_citation_args(&args);
		// The macro syntax should pass through the arguments, validation happens in constructor
		assert!(result.is_some());
	}

	#[test]
	fn test_parse_non_git_source() {
		let args: Vec<Expr> = vec![parse_quote!(http), parse_quote!(url = "https://example.com")];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_none());
	}

	#[test]
	fn test_parse_git_with_single_line() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
			parse_quote!(path = "README.md#L5"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "README.md");
		assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
	}

	#[test]
	fn test_parse_git_with_complex_path() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(revision = "74aa653664cd90adcc5f836f1777f265c109045b"),
			parse_quote!(path = "src/core/behavior.rs#L42-L100"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "src/core/behavior.rs");
		assert_eq!(git_source.comparison_revision, "74aa653664cd90adcc5f836f1777f265c109045b");
	}
}
