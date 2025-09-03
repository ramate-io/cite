//! Git Source Implementation for Citation Macros
//!
//! This module provides parsing and construction of GitSource sources within
//! the `#[cite]` procedural macro. It handles the translation from macro syntax
//! to actual GitSource objects that can validate git content at compile time.
//!
//! # Design Rationale
//!
//! Git sources enable citations to validate git repository content:
//!
//! 1. **Cross-Repository Validation**: Ensure content in other repositories hasn't changed
//! 2. **Version Tracking**: Track changes in specific commits or branches
//! 3. **Dependency Validation**: Verify that referenced code hasn't been modified
//!
//! # Syntax Design
//!
//! The Git syntax follows the keyword argument pattern:
//!
//! ```rust,ignore
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "README.md")]
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/lib.rs#L1-L10")]
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/**/*.rs")]
//! ```
//!
//! This syntax was chosen because:
//! - **Clear Intent**: Revision and path are explicitly separated
//! - **Type Safety**: Different path types (file, line range, glob pattern)
//! - **Extensibility**: Easy to add new path types and git options
//!
//! # Implementation Strategy
//!
//! The module uses a multi-phase approach:
//!
//! 1. **Syntax Parsing**: Extract Git-specific arguments from the citation
//! 2. **Validation**: Validate revision format and path syntax at compile time
//! 3. **Source Construction**: Create GitSource instances using cite-git
//!
//! This separation allows the parsing logic to focus on syntax while delegating
//! the actual Git source creation to the git library.

use cite_git::GitSource;

/// Try to construct a GitSource from kwargs
///
/// Supports syntax like:
/// - `remote = "https://github.com/ramate-io/cite", ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", cur_rev = "main", path = "README.md"` -> GitSource for file
/// - `remote = "https://github.com/ramate-io/cite", ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", cur_rev = "main", path = "src/lib.rs#L1-L10"` -> GitSource with line range
/// - `remote = "https://github.com/ramate-io/cite", ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", cur_rev = "main", path = "src/**/*.rs"` -> GitSource with glob pattern
/// - `remote = "https://github.com/ramate-io/cite", ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", cur_rev = "main", path = "README.md#L5"` -> GitSource with single line
pub fn try_get_git_source_from_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<GitSource, String> {
	// Extract required parameters
	let remote = kwargs
		.get("remote")
		.and_then(|v| v.as_str())
		.ok_or("Git source requires 'remote' parameter")?;

	let ref_rev = kwargs
		.get("ref_rev")
		.or_else(|| kwargs.get("referenced_revision"))
		.and_then(|v| v.as_str())
		.ok_or("Git source requires 'ref_rev' or 'referenced_revision' parameter")?;

	let cur_rev = kwargs
		.get("cur_rev")
		.or_else(|| kwargs.get("current_revision"))
		.and_then(|v| v.as_str())
		.ok_or("Git source requires 'cur_rev' or 'current_revision' parameter")?;

	let path = kwargs
		.get("path")
		.and_then(|v| v.as_str())
		.ok_or("Git source requires 'path' parameter")?;

	// Extract optional name parameter
	let name = kwargs.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());

	// Validate URL format
	if !is_valid_git_url(remote) {
		return Err(format!("Invalid Git remote URL format: {}", remote));
	}

	// Construct the GitSource with the appropriate parameters
	GitSource::try_new(remote, path, ref_rev, cur_rev, name)
		.map_err(|e| format!("Failed to create Git source: {:?}", e))
}

/// Basic Git URL validation for parse-time checking
fn is_valid_git_url(url: &str) -> bool {
	url.starts_with("https://") || url.starts_with("http://") || url.starts_with("git@")
}
