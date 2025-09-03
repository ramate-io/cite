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
//! [git, remote = "https://github.com/ramate-io/cite", referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", current_revision = "main", path = "README.md"]
//! [git, remote = "https://github.com/ramate-io/cite", referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", current_revision = "main", path = "src/lib.rs#L1-L10"]
//! [git, remote = "https://github.com/ramate-io/cite", referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", current_revision = "main", path = "src/**/*.rs"]
//! ```
//!
//! The parsing algorithm:
//! 1. Verify first argument is the identifier `git`
//! 2. Scan remaining arguments for assignment expressions
//! 3. Extract required `remote`, `referenced_revision`, `current_revision`, and `path` parameters
//! 4. Match additional parameters to known Git options
//! 5. Construct GitSource using cite-git
//!
//! # Supported Parameters
//!
//! **Required:**
//! - `remote = "https://github.com/ramate-io/cite"` - Remote repository URL
//! - `referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"` - Git commit hash or reference for the cited content
//! - `current_revision = "main"` - Git commit hash or reference for the current content to compare against
//! - `path = "README.md"` - File path relative to repository root
//!
//! **Optional:**
//! - `name = "My Custom Name"` - Custom name for the source (if not provided, a default name is generated)
//!
//! **Path Options:**
//! - `path = "src/lib.rs#L1-L10"` - File with line range specification
//! - `path = "src/**/*.rs"` - Glob pattern for multiple files
//! - `path = "README.md#L5"` - Single line specification
//!
//! # Error Handling
//!
//! The parser is designed to fail gracefully:
//! - Returns `None` if the syntax doesn't match Git source patterns
//! - Allows the main citation parser to try other source types
//! - Validation happens in the GitSource constructor, not in the macro
//! - Provides helpful error messages for malformed syntax
//!
//! # Examples
//!
//! ```rust,ignore
//! // Basic file validation
//! #[cite(git, remote = "https://github.com/ramate-io/cite",
//!        referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
//!        current_revision = "main",
//!        path = "README.md")]
//!
//! // Line range validation with custom name
//! #[cite(git, remote = "https://github.com/ramate-io/cite",
//!        referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
//!        current_revision = "main",
//!        path = "src/lib.rs#L1-L10",
//!        name = "Core Library Functions")]
//!
//! // Glob pattern validation
//! #[cite(git, remote = "https://github.com/ramate-io/cite",
//!        referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
//!        current_revision = "main",
//!        path = "src/**/*.rs")]
//!
//! // Single line validation
//! #[cite(git, remote = "https://github.com/ramate-io/cite",
//!        referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
//!        current_revision = "main",
//!        path = "README.md#L5")]
//! ```

use cite_git::GitSource;
use syn::{Expr, Lit};

/// Parse the keyword argument syntax for Git sources
///
/// Supports syntax like:
/// - `git, remote = "https://github.com/ramate-io/cite", referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", current_revision = "main", path = "README.md"`
/// - `git, remote = "https://github.com/ramate-io/cite", referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", current_revision = "main", path = "src/lib.rs#L1-L10"`
/// - `git, remote = "https://github.com/ramate-io/cite", referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", current_revision = "main", path = "src/**/*.rs"`
pub fn try_parse_from_citation_args(args: &[Expr]) -> Option<GitSource> {
	// Look for pattern: git, remote = "...", referenced_revision = "...", current_revision = "...", path = "..."
	// OR: source = "git", remote = "...", referenced_revision = "...", current_revision = "...", path = "..."
	if args.is_empty() {
		return None;
	}

	// Check if this is the old syntax (first argument is "git") or new syntax (has "source = "git"")
	let is_git_source = if let Expr::Path(path_expr) = &args[0] {
		let segment = &path_expr.path.segments[0];
		path_expr.path.segments.len() == 1 && segment.ident == "git"
	} else {
		false
	};

	let has_source_git = args.iter().any(|arg| {
		if let Expr::Assign(assign_expr) = arg {
			if let Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 && left_path.path.segments[0].ident == "source" {
					if let Expr::Lit(lit_expr) = &*assign_expr.right {
						if let syn::Lit::Str(str_lit) = &lit_expr.lit {
							return str_lit.value() == "git";
						}
					}
				}
			}
		}
		false
	});

	if !is_git_source && !has_source_git {
		return None;
	}

	// Look through arguments for assignments
	let mut remote = None;
	let mut referenced_revision = None;
	let mut current_revision = None;
	let mut path = None;
	let mut optional_name = None;

	for arg in args {
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
						"referenced_revision" | "ref_rev" => {
							if let Some(revision_str) =
								extract_string_literal(&assign_expr.right)
							{
								referenced_revision = Some(revision_str);
							}
						}
						"current_revision" | "cur_rev" => {
							if let Some(revision_str) =
								extract_string_literal(&assign_expr.right)
							{
								current_revision = Some(revision_str);
							}
						}
						"path" => {
							if let Some(path_str) =
								extract_string_literal(&assign_expr.right)
							{
								path = Some(path_str);
							}
						}
						"name" => {
							if let Some(name_str) =
								extract_string_literal(&assign_expr.right)
							{
								optional_name = Some(name_str);
							}
						}
						_ => continue, // Unknown parameter, skip
					}
				}
			}
		}
	}

	// Construct GitSource if we have required parameters
	if let (
		Some(remote_str),
		Some(referenced_revision_str),
		Some(current_revision_str),
		Some(path_str),
	) = (remote, referenced_revision, current_revision, path)
	{
		// Use the constructor for macro usage - let it handle validation
		return GitSource::try_new(
			&remote_str,
			&path_str,
			&referenced_revision_str,
			&current_revision_str,
			optional_name,
		)
		.ok();
	}

	// If we got this far but don't have required params, return None
	// This allows the main parser to show the proper error message
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
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
			parse_quote!(path = "README.md"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "README.md");
		assert_eq!(git_source.referenced_revision, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(git_source.current_revision, "main");
	}

	#[test]
	fn test_parse_git_with_line_range() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
			parse_quote!(path = "src/lib.rs#L1-L10"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "src/lib.rs");
		assert_eq!(git_source.referenced_revision, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(git_source.current_revision, "main");
	}

	#[test]
	fn test_parse_git_with_glob_pattern() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
			parse_quote!(path = "src/**/*.rs"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "src/**/*.rs");
		assert_eq!(git_source.referenced_revision, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(git_source.current_revision, "main");
	}

	#[test]
	fn test_parse_git_missing_referenced_revision() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(current_revision = "main"),
			parse_quote!(path = "README.md"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_none());
	}

	#[test]
	fn test_parse_git_missing_current_revision() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
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
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_none());
	}

	#[test]
	fn test_parse_git_invalid_revision() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(referenced_revision = "invalid"),
			parse_quote!(current_revision = "main"),
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
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
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
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
			parse_quote!(path = "README.md#L5"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "README.md");
		assert_eq!(git_source.referenced_revision, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(git_source.current_revision, "main");
	}

	#[test]
	fn test_parse_git_with_complex_path() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
			parse_quote!(path = "src/core/behavior.rs#L42-L100"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "src/core/behavior.rs");
		assert_eq!(git_source.referenced_revision, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(git_source.current_revision, "main");
	}

	#[test]
	fn test_parse_git_with_abbreviated_syntax() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(cur_rev = "main"),
			parse_quote!(path = "README.md"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "README.md");
		assert_eq!(git_source.referenced_revision, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(git_source.current_revision, "main");
	}

	#[test]
	fn test_parse_git_with_name_parameter() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
			parse_quote!(path = "README.md"),
			parse_quote!(name = "My Custom Name"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "README.md");
		assert_eq!(git_source.referenced_revision, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(git_source.current_revision, "main");
		assert_eq!(git_source.name, "My Custom Name");
	}

	#[test]
	fn test_parse_git_without_name_parameter() {
		let args: Vec<Expr> = vec![
			parse_quote!(git),
			parse_quote!(remote = "https://github.com/ramate-io/cite"),
			parse_quote!(referenced_revision = "94dab273cf6c2abe8742d6d459ad45c96ca9b694"),
			parse_quote!(current_revision = "main"),
			parse_quote!(path = "README.md"),
		];

		let result = try_parse_from_citation_args(&args);
		assert!(result.is_some());

		let git_source = result.unwrap();
		assert_eq!(git_source.remote, "https://github.com/ramate-io/cite");
		assert_eq!(git_source.path_pattern.path, "README.md");
		assert_eq!(git_source.referenced_revision, "94dab273cf6c2abe8742d6d459ad45c96ca9b694");
		assert_eq!(git_source.current_revision, "main");
		// When no name is provided, it should use the default generated name
		assert!(git_source.name.contains("https://github.com/ramate-io/cite"));
		assert!(git_source.name.contains("README.md"));
		assert!(git_source.name.contains("94dab273cf6c2abe8742d6d459ad45c96ca9b694"));
	}
}
